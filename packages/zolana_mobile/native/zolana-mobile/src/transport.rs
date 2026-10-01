//! The application's HTTP client. A wallet opened with a [`Transport`] sends
//! every request through it, Solana RPC and indexer calls and proving-key
//! downloads, and opens no connection of its own.

use std::{collections::HashMap, fmt, sync::Arc};

use flutter_rust_bridge::DartFnFuture;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    ResponseBuilderExt, StatusCode,
};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next};
use zolana_api::{ApiError, BlockingHttpClient, HttpResponse};

/// One HTTP request of the wallet.
pub struct TransportRequest {
    /// `POST` for Solana RPC and indexer calls, `GET` for proving keys.
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    /// Empty for a `GET`.
    pub body: Vec<u8>,
}

/// The server's response, whatever its status.
pub struct TransportResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Sends the wallet's requests through the application.
#[derive(Clone)]
pub struct Transport {
    callback: Arc<
        dyn Fn(TransportRequest) -> DartFnFuture<anyhow::Result<TransportResponse>> + Send + Sync,
    >,
}

impl Transport {
    /// `send` answers a request with the server's response, and fails only
    /// when there is none. The wallet waits for it, so it must not call the
    /// wallet.
    pub fn new(
        send: impl Fn(TransportRequest) -> DartFnFuture<anyhow::Result<TransportResponse>>
            + Send
            + Sync
            + 'static,
    ) -> Transport {
        Transport {
            callback: Arc::new(send),
        }
    }

    async fn send(&self, request: TransportRequest) -> Result<TransportResponse, TransportFailed> {
        (self.callback)(request).await.map_err(TransportFailed::new)
    }

    /// Blocks until the application answers. The wallet's methods run on a
    /// worker thread and the application answers on its own event loop, so
    /// waiting here never holds up the answer.
    pub(crate) fn send_blocking(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, TransportFailed> {
        futures_executor::block_on(self.send(request))
    }

    /// The HTTP client of the Solana RPC client: `client` builds each request,
    /// and this transport sends it with `headers` added.
    pub(crate) fn solana_rpc(
        &self,
        client: reqwest::Client,
        headers: HeaderMap,
    ) -> ClientWithMiddleware {
        ClientBuilder::new(client)
            .with(WithHeaders {
                transport: self.clone(),
                headers,
            })
            .build()
    }

    /// The HTTP client of the indexer, with `headers` on every request.
    pub(crate) fn indexer(&self, headers: HeaderMap) -> impl BlockingHttpClient {
        WithHeaders {
            transport: self.clone(),
            headers,
        }
    }
}

/// The application's transport failed without a response.
#[derive(Debug)]
pub(crate) struct TransportFailed(String);

impl TransportFailed {
    /// The Dart exception, without the stack trace the bridge appends after a
    /// blank line.
    fn new(error: anyhow::Error) -> Self {
        let message = error.to_string();
        let exception = message.split("\n\n").next().unwrap_or_default();
        Self(exception.to_string())
    }
}

impl fmt::Display for TransportFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transport failed: {}", self.0)
    }
}

impl std::error::Error for TransportFailed {}

/// A transport with the headers one service adds to every request. The
/// request's own headers win, as they do over a `reqwest` client's defaults.
struct WithHeaders {
    transport: Transport,
    headers: HeaderMap,
}

impl WithHeaders {
    fn request(
        &self,
        method: &str,
        url: &str,
        headers: &HeaderMap,
        body: Vec<u8>,
    ) -> TransportRequest {
        let mut merged = HashMap::new();
        for (name, value) in headers.iter().chain(&self.headers) {
            merged
                .entry(name.to_string())
                .or_insert_with(|| String::from_utf8_lossy(value.as_bytes()).into_owned());
        }
        TransportRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: merged,
            body,
        }
    }
}

impl fmt::Debug for WithHeaders {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WithHeaders")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl Middleware for WithHeaders {
    async fn handle(
        &self,
        request: reqwest::Request,
        _: &mut http::Extensions,
        _: Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let body = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .unwrap_or_default()
            .to_vec();
        let url = request.url();
        let response = self
            .transport
            .send(self.request(
                request.method().as_str(),
                url.as_str(),
                request.headers(),
                body,
            ))
            .await
            .map_err(reqwest_middleware::Error::middleware)?;
        let response = http::Response::builder()
            .status(response.status)
            .url(url.clone())
            .body(response.body)
            .map_err(reqwest_middleware::Error::middleware)?;
        Ok(response.into())
    }
}

impl BlockingHttpClient for WithHeaders {
    fn post_json(&self, url: &str, body: Vec<u8>) -> Result<HttpResponse, ApiError> {
        let json =
            HeaderMap::from_iter([(CONTENT_TYPE, HeaderValue::from_static("application/json"))]);
        let response = self
            .transport
            .send_blocking(self.request("POST", url, &json, body))
            .map_err(|error| ApiError::HttpClient(Box::new(error)))?;
        let status = StatusCode::from_u16(response.status)
            .map_err(|error| ApiError::HttpClient(Box::new(error)))?;
        Ok(HttpResponse {
            status,
            body: String::from_utf8_lossy(&response.body).into_owned(),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use solana_keypair::Keypair;
    use solana_signer::Signer;

    use super::*;
    use crate::{derivation_message, MobileWallet, WalletConfig};

    pub(crate) type Requests = Arc<Mutex<Vec<TransportRequest>>>;

    /// A transport that answers each request with `answer` and records it.
    pub(crate) fn fake(
        answer: impl Fn(&TransportRequest) -> anyhow::Result<TransportResponse> + Send + Sync + 'static,
    ) -> (Transport, Requests) {
        let requests = Requests::default();
        let recorded = requests.clone();
        let transport = Transport::new(move |request| {
            let response = answer(&request);
            recorded.lock().unwrap().push(request);
            Box::pin(async move { response })
        });
        (transport, requests)
    }

    pub(crate) fn ok(body: &str) -> anyhow::Result<TransportResponse> {
        Ok(TransportResponse {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    const RPC_URL: &str = "https://rpc.example/?api-key=secret";
    const INDEXER_URL: &str = "https://indexer.example/v1/zolana?api-key=secret";

    fn open(signer: &Keypair, transport: Transport) -> MobileWallet {
        let header = |name: &str, value: &str| Some(HashMap::from([(name.into(), value.into())]));
        let config = WalletConfig {
            rpc_url: RPC_URL.to_string(),
            rpc_headers: header("x-rpc-token", "rpc-secret"),
            indexer_url: INDEXER_URL.to_string(),
            indexer_headers: header("x-indexer-token", "indexer-secret"),
            proving_key_dir: std::env::temp_dir().display().to_string(),
            proving_key_url: None,
            proving: None,
            allow_insecure_http: false,
            mints: Vec::new(),
        };
        let pubkey = signer.pubkey().to_string();
        let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
        MobileWallet::open(config, pubkey, signature.as_ref().to_vec(), Some(transport)).unwrap()
    }

    fn json(body: &[u8]) -> serde_json::Value {
        serde_json::from_slice(body).unwrap()
    }

    #[test]
    fn sends_rpc_and_indexer_requests_through_the_transport() {
        // An empty page of every indexer method the wallet reads.
        let (transport, requests) = fake(|request| {
            ok(if request.url == RPC_URL {
                r#"{"jsonrpc":"2.0","id":0,"result":{"context":{"slot":1},"value":null}}"#
            } else {
                r#"{"jsonrpc":"2.0","id":"test-account","result":{"context":{"blockTime":0,"slot":1},"matches":[],"transactions":[],"nextCursor":null}}"#
            })
        });
        let mut wallet = open(&Keypair::new(), transport);

        assert_eq!(
            wallet.registration_status(),
            Ok(crate::RegistrationStatus::NotRegistered)
        );
        let rpc = requests.lock().unwrap().pop().unwrap();
        assert_eq!((rpc.method.as_str(), rpc.url.as_str()), ("POST", RPC_URL));
        assert_eq!(rpc.headers["x-rpc-token"], "rpc-secret");
        assert_eq!(rpc.headers["content-type"], "application/json");
        assert!(!rpc.headers.contains_key("x-indexer-token"));
        assert_eq!(json(&rpc.body)["method"], "getAccountInfo");

        assert_eq!(wallet.private_balance(None), Ok(0));
        let indexer = std::mem::take(&mut *requests.lock().unwrap());
        assert!(!indexer.is_empty());
        for request in indexer {
            let method = json(&request.body)["method"].as_str().unwrap().to_string();
            assert_eq!(request.method, "POST");
            assert_eq!(
                request.url,
                format!("https://indexer.example/v1/zolana/{method}?api-key=secret")
            );
            assert_eq!(request.headers["x-indexer-token"], "indexer-secret");
            assert_eq!(request.headers["content-type"], "application/json");
            assert!(!request.headers.contains_key("x-rpc-token"));
        }
    }

    #[test]
    fn transport_failures_surface_without_secrets() {
        // The bridge hands over the Dart exception and its stack trace.
        let (transport, _) = fake(|request| {
            Err(anyhow::anyhow!(
                "SocketException: connection refused ({})\n\n#0 frame",
                request.url
            ))
        });
        let mut wallet = open(&Keypair::new(), transport);
        for error in [
            wallet.registration_status().unwrap_err(),
            wallet.private_balance(None).unwrap_err(),
        ] {
            assert!(
                error.contains("transport failed: SocketException"),
                "{error}"
            );
            assert!(error.contains("api-key=redacted"), "{error}");
            assert!(
                !error.contains("secret") && !error.contains("#0"),
                "{error}"
            );
        }

        let (transport, _) = fake(|_| {
            Ok(TransportResponse {
                status: 401,
                body: b"unauthorized".to_vec(),
            })
        });
        let mut wallet = open(&Keypair::new(), transport);
        for error in [
            wallet.registration_status().unwrap_err(),
            wallet.private_balance(None).unwrap_err(),
        ] {
            assert!(error.contains("401"), "{error}");
            assert!(!error.contains("secret"), "{error}");
        }
    }
}
