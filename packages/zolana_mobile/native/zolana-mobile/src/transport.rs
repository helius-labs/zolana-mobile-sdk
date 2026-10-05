//! The application's HTTP client, the wallet's only network path. Solana RPC
//! and indexer calls, proving-key downloads and backend proofs all go through
//! it; the wallet opens no connection of its own.

use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use flutter_rust_bridge::DartFnFuture;
use reqwest::{header::HeaderMap, ResponseBuilderExt, StatusCode};
use reqwest_middleware::{ClientBuilder, Middleware, Next};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::{
    http_sender::HttpSender,
    rpc_client::{RpcClient, RpcClientConfig},
};
use zolana_api::{ApiError, BlockingHttpClient, BlockingZolanaApi, HttpRequest, HttpResponse};
use zolana_client::{ProverClient, SolanaRpc, ZolanaIndexer};

use crate::error::WalletError;

/// One HTTP request of the wallet.
pub struct TransportRequest {
    /// `POST` for Solana RPC and indexer calls and proof requests, `GET` for
    /// proving keys and the status of a queued proof.
    pub method: String,
    pub url: String,
    /// The content type of a `POST`, and `x-sync` or `x-async` on a proof
    /// request; nothing else.
    pub headers: HashMap<String, String>,
    /// Empty for a `GET`. A proof request's body is the transaction's
    /// witness, the wallet's nullifier secret included.
    pub body: Vec<u8>,
    /// The most bytes the response body may hold, for a proving-key download
    /// (the key's size in the lockfile). A transport stops reading and fails
    /// past it; the wallet refuses a longer body either way.
    pub max_response_bytes: Option<u32>,
    /// The caller's bound on the request, in milliseconds, when it has one
    /// (600 s for a proof request, 30 s for a status poll); otherwise the
    /// transport's own applies.
    pub timeout_ms: Option<u32>,
}

/// The server's response, whatever its status.
pub struct TransportResponse {
    pub status: u16,
    /// The response's headers as they arrived. A prover inside a TEE says in
    /// them that its body is encrypted; a TEE call fails without them.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// What the application's transport answered: the server's response, or the
/// failure that left none. The package's Dart side builds it and never
/// throws, so no Dart stack trace reaches the wallet.
pub struct TransportOutcome {
    pub response: Option<TransportResponse>,
    pub failure: Option<TransportFailure>,
}

/// Why the application's transport has no response.
#[derive(Debug)]
pub struct TransportFailure {
    pub message: String,
    /// The response's status, when it arrived and its body could not be read.
    /// The server got the request and may have acted on it, so the wallet does
    /// not send it again: a prover would prove the spend twice.
    pub status: Option<u16>,
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
        outcome.response.ok_or_else(|| {
            TransportFailed(outcome.failure.unwrap_or(TransportFailure {
                message: String::new(),
                status: None,
            }))
        })
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
    pub(crate) fn solana_rpc(&self, url: String) -> Result<SolanaRpc, WalletError> {
        // The middleware stack needs a base client. The sender below answers
        // every request itself, so the base never sends.
        let base =
            reqwest::Client::builder()
                .build()
                .map_err(|failure| WalletError::TransportFailed {
                    message: failure.to_string(),
                })?;
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

    /// The SDK's prover client of the prover at `url`, through this transport.
    pub(crate) fn prover(&self, url: String) -> ProverClient {
        ProverClient::with_client(url, Sender(self.clone()))
    }
}

/// The application's transport failed without a response.
#[derive(Debug)]
pub(crate) struct TransportFailed(TransportFailure);

impl fmt::Display for TransportFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let TransportFailure { message, status } = &self.0;
        match status {
            Some(status) => write!(
                formatter,
                "transport failed after status {status}: {message}"
            ),
            None => write!(formatter, "transport failed: {message}"),
        }
    }
}

impl std::error::Error for TransportFailed {}

/// The HTTP client the SDK's clients call: each request becomes a
/// [`TransportRequest`] with its headers as they are.
struct Sender(Transport);

fn header_map(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(name, value)| {
            let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
            (name.to_string(), value)
        })
        .collect()
}

fn millis(timeout: Duration) -> u32 {
    u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX)
}

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
        let headers = header_map(request.headers());
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
                timeout_ms: None,
            })
            .await
            .map_err(reqwest_middleware::Error::middleware)?;
        let mut builder = http::Response::builder().status(response.status).url(url);
        if let Some(headers) = builder.headers_mut() {
            *headers = response_headers(response.headers);
        }
        let response = builder
            .body(response.body)
            .map_err(reqwest_middleware::Error::middleware)?;
        Ok(response.into())
    }
}

/// A failure after the status arrived is [`ApiError::ResponseLost`], which the
/// prover client does not retry; any other is [`ApiError::HttpClient`].
impl BlockingHttpClient for Sender {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, ApiError> {
        let HttpRequest {
            method,
            url,
            headers,
            mut body,
            timeout,
            body_limit,
        } = request;
        let request = TransportRequest {
            method: method.to_string(),
            url,
            headers: header_map(&headers),
            // Moved, not copied: the bridge hands it to the application.
            body: std::mem::take(&mut *body),
            max_response_bytes: body_limit.map(|limit| u32::try_from(limit).unwrap_or(u32::MAX)),
            timeout_ms: timeout.map(millis),
        };
        let response = self.0.send_blocking(request).map_err(|error| {
            if error.0.status.is_some() {
                ApiError::ResponseLost(Box::new(error))
            } else {
                ApiError::HttpClient(Box::new(error))
            }
        })?;
        let status = StatusCode::from_u16(response.status)
            .map_err(|error| ApiError::HttpClient(Box::new(error)))?;
        Ok(HttpResponse {
            status,
            headers: response_headers(response.headers),
            body: response.body,
        })
    }
}

/// The headers a transport returned, without any that are not valid HTTP.
fn response_headers(headers: HashMap<String, String>) -> HeaderMap {
    headers
        .into_iter()
        .filter_map(|(name, value)| {
            Some((
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).ok()?,
                reqwest::header::HeaderValue::from_str(&value).ok()?,
            ))
        })
        .collect()
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
    /// failure, and records it.
    pub(crate) fn fake(
        answer: impl Fn(&TransportRequest) -> Result<TransportResponse, TransportFailure>
            + Send
            + Sync
            + 'static,
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
        fake(|request| refused(format!("connection refused ({})", request.url))).0
    }

    /// A failure before any response.
    pub(crate) fn refused(message: String) -> Result<TransportResponse, TransportFailure> {
        Err(TransportFailure {
            message,
            status: None,
        })
    }

    pub(crate) fn ok(body: &str) -> Result<TransportResponse, TransportFailure> {
        Ok(TransportResponse {
            status: 200,
            headers: Default::default(),
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
            prover_url: None,
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
        assert_eq!((rpc.max_response_bytes, rpc.timeout_ms), (None, None));
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
    fn sends_the_sdk_request_as_it_is() {
        let (transport, requests) = fake(|_| {
            Ok(TransportResponse {
                status: 200,
                headers: HashMap::from([
                    ("x-tee-encrypted".to_string(), "1".to_string()),
                    ("bad header".to_string(), "dropped".to_string()),
                ]),
                body: b"{}".to_vec(),
            })
        });
        let request = HttpRequest::post_json("https://indexer.example/m", b"{}".to_vec())
            .with_header(
                "x-request".parse().unwrap(),
                reqwest::header::HeaderValue::from_static("1"),
            )
            .with_timeout(Duration::from_millis(2500));
        let response = Sender(transport).send(request).unwrap();
        assert_eq!(
            (response.status.as_u16(), response.body.as_slice()),
            (200, &b"{}"[..])
        );
        // The response headers reach the SDK, without any that are not HTTP.
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers["x-tee-encrypted"], "1");
        let sent = requests.lock().unwrap().pop().unwrap();
        assert_eq!(
            (sent.method.as_str(), sent.url.as_str()),
            ("POST", "https://indexer.example/m")
        );
        let mut headers = json_content_type();
        headers.insert("x-request".into(), "1".into());
        assert_eq!(sent.headers, headers);
        assert_eq!(sent.body, b"{}");
        assert_eq!(
            (sent.max_response_bytes, sent.timeout_ms),
            (None, Some(2500))
        );
    }

    #[test]
    fn a_failure_after_the_status_is_a_lost_response() {
        let send = |status| {
            let (transport, _) = fake(move |_| {
                Err(TransportFailure {
                    message: "connection reset".to_string(),
                    status,
                })
            });
            Sender(transport).send(HttpRequest::get("https://prover.example/status"))
        };
        assert!(matches!(
            send(Some(200)),
            Err(ApiError::ResponseLost(error))
                if error.to_string() == "transport failed after status 200: connection reset"
        ));
        assert!(matches!(
            send(None),
            Err(ApiError::HttpClient(error))
                if error.to_string() == "transport failed: connection reset"
        ));
    }

    #[test]
    fn transport_failures_surface_without_secrets() {
        // The package hands over the exception message, which can name the URL.
        let (transport, _) = fake(|request| {
            refused(format!(
                "SocketException: connection refused ({})",
                request.url
            ))
        });
        let mut wallet = open(&Keypair::new(), transport);
        for error in [
            wallet.registration_status().unwrap_err(),
            wallet.private_balance(None).unwrap_err(),
        ] {
            let WalletError::Client { message: error } = error else {
                panic!("{error:?}");
            };
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
                headers: Default::default(),
                body: b"unauthorized".to_vec(),
            })
        });
        let mut wallet = open(&Keypair::new(), transport);
        for error in [
            wallet.registration_status().unwrap_err(),
            wallet.private_balance(None).unwrap_err(),
        ] {
            let WalletError::Client { message: error } = error else {
                panic!("{error:?}");
            };
            assert!(error.contains("401"), "{error}");
            assert!(!error.contains("secret"), "{error}");
        }
    }
}
