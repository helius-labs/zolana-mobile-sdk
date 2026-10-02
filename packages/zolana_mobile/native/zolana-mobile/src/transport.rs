//! The application's HTTP client, the wallet's only network path. Solana RPC
//! and indexer calls and proving-key downloads all go through it; the wallet
//! opens no connection of its own.

use std::{collections::HashMap, fmt, sync::Arc};

use flutter_rust_bridge::DartFnFuture;
use reqwest::{ResponseBuilderExt, StatusCode};
use reqwest_middleware::{ClientBuilder, Middleware, Next};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::{
    http_sender::HttpSender,
    rpc_client::{RpcClient, RpcClientConfig},
};
use zolana_api::{ApiError, BlockingHttpClient, BlockingZolanaApi, HttpResponse};
use zolana_client::{SolanaRpc, ZolanaIndexer};

/// One HTTP request of the wallet.
pub struct TransportRequest {
    /// `POST` for Solana RPC and indexer calls, `GET` for proving keys.
    pub method: String,
    pub url: String,
    /// The content type of a `POST`; nothing else.
    pub headers: HashMap<String, String>,
    /// Empty for a `GET`.
    pub body: Vec<u8>,
    /// The most bytes the response body may hold, for a proving-key download
    /// (the key's size in the lockfile). A transport stops reading and fails
    /// past it; the wallet refuses a longer body either way.
    pub max_response_bytes: Option<u32>,
}

/// The server's response, whatever its status.
pub struct TransportResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// What the application's transport answered: the server's response, or the
/// message of the failure that left none. The package's Dart side builds it
/// and never throws, so no Dart stack trace reaches the wallet.
pub struct TransportOutcome {
    pub response: Option<TransportResponse>,
    pub failure: Option<String>,
}

/// Sends the wallet's requests through the application.
#[derive(Clone)]
pub struct Transport {
    callback: Arc<dyn Fn(TransportRequest) -> DartFnFuture<TransportOutcome> + Send + Sync>,
}

impl Transport {
    /// `send` answers a request with the server's response, or with a failure
    /// when there is none. The wallet waits for it, so it must not call the
    /// wallet.
    pub fn new(
        send: impl Fn(TransportRequest) -> DartFnFuture<TransportOutcome> + Send + Sync + 'static,
    ) -> Transport {
        Transport {
            callback: Arc::new(send),
        }
    }

    async fn send(&self, request: TransportRequest) -> Result<TransportResponse, TransportFailed> {
        let outcome = (self.callback)(request).await;
        outcome
            .response
            .ok_or_else(|| TransportFailed(outcome.failure.unwrap_or_default()))
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

    /// The Solana RPC client of `url`: solana-rpc-client's own sender, whose
    /// requests this transport answers, so its JSON-RPC parsing, error data
    /// and retries stay.
    pub(crate) fn solana_rpc(&self, url: String) -> Result<SolanaRpc, String> {
        // The middleware stack needs a base client. The sender below answers
        // every request itself, so the base never sends.
        let base = reqwest::Client::builder()
            .build()
            .map_err(|_| "rpc_client_unavailable".to_string())?;
        let client = ClientBuilder::new(base).with(Sender(self.clone())).build();
        Ok(SolanaRpc::with_client(RpcClient::new_sender(
            HttpSender::new_with_client_with_middleware(url, client),
            RpcClientConfig::with_commitment(CommitmentConfig::confirmed()),
        )))
    }

    /// The indexer client of `url`, through this transport.
    pub(crate) fn indexer(&self, url: &str) -> ZolanaIndexer {
        ZolanaIndexer::with_api(BlockingZolanaApi::with_client(url, Sender(self.clone())))
    }
}

/// The application's transport failed without a response.
#[derive(Debug)]
pub(crate) struct TransportFailed(String);

impl fmt::Display for TransportFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transport failed: {}", self.0)
    }
}

impl std::error::Error for TransportFailed {}

/// The HTTP client the SDK's clients call: each request becomes a
/// [`TransportRequest`] with its headers as they are.
struct Sender(Transport);

impl fmt::Debug for Sender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Sender")
    }
}

#[async_trait::async_trait]
impl Middleware for Sender {
    async fn handle(
        &self,
        request: reqwest::Request,
        _: &mut http::Extensions,
        _: Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let url = request.url().clone();
        let headers = request
            .headers()
            .iter()
            .map(|(name, value)| {
                let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
                (name.to_string(), value)
            })
            .collect();
        let body = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .unwrap_or_default()
            .to_vec();
        let response = self
            .0
            .send(TransportRequest {
                method: request.method().to_string(),
                url: url.to_string(),
                headers,
                body,
                max_response_bytes: None,
            })
            .await
            .map_err(reqwest_middleware::Error::middleware)?;
        let response = http::Response::builder()
            .status(response.status)
            .url(url)
            .body(response.body)
            .map_err(reqwest_middleware::Error::middleware)?;
        Ok(response.into())
    }
}

impl BlockingHttpClient for Sender {
    fn post_json(&self, url: &str, body: Vec<u8>) -> Result<HttpResponse, ApiError> {
        let request = TransportRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body,
            max_response_bytes: None,
        };
        let response = self
            .0
            .send_blocking(request)
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

    /// A transport that answers each request with `answer`, a response or a
    /// failure message, and records it.
    pub(crate) fn fake(
        answer: impl Fn(&TransportRequest) -> Result<TransportResponse, String> + Send + Sync + 'static,
    ) -> (Transport, Requests) {
        let requests = Requests::default();
        let recorded = requests.clone();
        let transport = Transport::new(move |request| {
            let (response, failure) = match answer(&request) {
                Ok(response) => (Some(response), None),
                Err(failure) => (None, Some(failure)),
            };
            recorded.lock().unwrap().push(request);
            Box::pin(async move { TransportOutcome { response, failure } })
        });
        (transport, requests)
    }

    /// A transport with no network: every request fails as a refused
    /// connection does, naming the URL as the application's exception would.
    pub(crate) fn unreachable() -> Transport {
        fake(|request| Err(format!("connection refused ({})", request.url))).0
    }

    pub(crate) fn ok(body: &str) -> Result<TransportResponse, String> {
        Ok(TransportResponse {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    const RPC_URL: &str = "https://rpc.example/?api-key=secret";
    const INDEXER_URL: &str = "https://indexer.example/v1/zolana?api-key=secret";

    fn open(signer: &Keypair, transport: Transport) -> MobileWallet {
        let config = WalletConfig {
            rpc_url: RPC_URL.to_string(),
            indexer_url: INDEXER_URL.to_string(),
            proving_key_dir: std::env::temp_dir().display().to_string(),
            proving_key_url: None,
            proving: None,
            mints: Vec::new(),
        };
        let pubkey = signer.pubkey().to_string();
        let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
        MobileWallet::open(config, pubkey, signature.as_ref().to_vec(), transport).unwrap()
    }

    fn json(body: &[u8]) -> serde_json::Value {
        serde_json::from_slice(body).unwrap()
    }

    fn json_content_type() -> HashMap<String, String> {
        HashMap::from([("content-type".to_string(), "application/json".to_string())])
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
        assert_eq!(rpc.headers, json_content_type());
        assert_eq!(rpc.max_response_bytes, None);
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
            assert_eq!(request.headers, json_content_type());
        }
    }

    #[test]
    fn transport_failures_surface_without_secrets() {
        // The package hands over the exception message, which can name the URL.
        let (transport, _) = fake(|request| {
            Err(format!(
                "SocketException: connection refused ({})",
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
            assert!(!error.contains("secret"), "{error}");
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
