//! A devnet integration example, not a production sponsorship service.
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_keypair::Keypair;
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, Semaphore};
use zolana_client::{compile_message, rpc::sign_transaction, ComputeBudgetConfig};
use zolana_interface::{
    instruction::{tag, CreateCacheData, MergeTransactIxData},
    pda,
};
use zolana_mobile::CacheTarget;
use zolana_program::instruction::{CacheWriteAccounts, CloseCache, CreateCache, MergeTransact};
use zolana_user_registry_interface::user_record_pda;

type Result<T> = std::result::Result<T, Failure>;
struct Failure(StatusCode, &'static str);
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}
fn bad(message: &'static str) -> Failure {
    Failure(StatusCode::BAD_REQUEST, message)
}
fn unavailable() -> Failure {
    Failure(
        StatusCode::BAD_GATEWAY,
        "upstream request failed; reconcile submission before retrying",
    )
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

struct App {
    token: String,
    sponsor: Keypair,
    prover: reqwest::Url,
    rpc_url: String,
    http: reqwest::Client,
    journal: PathBuf,
    journal_lock: Mutex<()>,
    concurrency: Semaphore,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    id: String,
    owner: String,
    target: CacheTarget,
    slots: u8,
    submissions: BTreeMap<u8, Submission>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Submission {
    message: String,
    transaction: String,
    signature: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartRun {
    owner: String,
    tree_id: u16,
    slots: u8,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Relay {
    slot: u8,
    message: String,
}

impl App {
    fn authorize(&self, headers: &HeaderMap) -> Result<()> {
        let supplied = headers
            .get("x-sf-auth")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        // Demo shared bearer token; production must bind runs to authenticated users.
        if !bool::from(supplied.as_bytes().ct_eq(self.token.as_bytes())) {
            return Err(Failure(StatusCode::UNAUTHORIZED, "invalid x-sf-auth"));
        }
        Ok(())
    }
    fn path(&self, id: &str) -> Result<PathBuf> {
        if id.len() != 16 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad("invalid run id"));
        }
        Ok(self.journal.join(format!("{id}.json")))
    }
    fn load(&self, id: &str) -> Result<Run> {
        let bytes = std::fs::read(self.path(id)?)
            .map_err(|_| Failure(StatusCode::NOT_FOUND, "run not found"))?;
        serde_json::from_slice(&bytes).map_err(|_| bad("invalid run journal"))
    }
    fn save(&self, run: &Run) -> Result<()> {
        use std::io::Write;
        let path = self.path(&run.id)?;
        let temporary = path.with_extension("tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| bad("cannot persist run"))?;
        file.write_all(&serde_json::to_vec(run).map_err(|_| bad("cannot encode run"))?)
            .map_err(|_| bad("cannot persist run"))?;
        file.sync_all().map_err(|_| bad("cannot sync run"))?;
        std::fs::rename(temporary, path).map_err(|_| bad("cannot commit run"))?;
        std::fs::File::open(&self.journal)
            .and_then(|d| d.sync_all())
            .map_err(|_| bad("cannot sync journal"))?;
        Ok(())
    }
    async fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        let result: Value = self
            .http
            .post(&self.rpc_url)
            .timeout(Duration::from_secs(30))
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
            .send()
            .await
            .map_err(|_| unavailable())?
            .error_for_status()
            .map_err(|_| unavailable())?
            .json()
            .await
            .map_err(|_| unavailable())?;
        if result.get("error").is_some() {
            return Err(unavailable());
        }
        result.get("result").cloned().ok_or_else(unavailable)
    }
}

async fn start(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(request): Json<StartRun>,
) -> Result<Json<Run>> {
    app.authorize(&headers)?;
    Pubkey::from_str(&request.owner).map_err(|_| bad("invalid owner"))?;
    if request.slots == 0 || request.slots > 4 {
        return Err(bad("example supports one to four merge slots"));
    }
    let _lock = app.journal_lock.lock().await;
    if std::fs::read_dir(&app.journal)
        .map_err(|_| bad("cannot read journal"))?
        .count()
        >= 128
    {
        return Err(Failure(
            StatusCode::TOO_MANY_REQUESTS,
            "example run quota reached",
        ));
    }
    let nonce = rand::random::<u64>() >> 1;
    let run = Run {
        id: format!("{nonce:016x}"),
        owner: request.owner,
        slots: request.slots,
        target: CacheTarget {
            sponsor: app.sponsor.pubkey().to_string(),
            nonce,
            tree_id: request.tree_id,
            expires_at: now() + 600,
            slot: 0,
            max_priority_fee_lamports: 10_000,
            compute_unit_limit: 400_000,
        },
        submissions: BTreeMap::new(),
    };
    app.save(&run)?;
    Ok(Json(run))
}

/// Reconstruct the only permitted message from trusted run terms and decoded
/// merge data, then require byte equality. This rejects extra instructions,
/// accounts, signers, altered privileges, version changes and header changes.
fn validate(run: &Run, slot: u8, bytes: &[u8]) -> Result<VersionedMessage> {
    if bytes.len() + 65 > 4096 || slot >= run.slots || now() >= run.target.expires_at {
        return Err(bad("run expired or transaction outside limits"));
    }
    let message: VersionedMessage =
        wincode::deserialize_exact(bytes).map_err(|_| bad("invalid message"))?;
    let VersionedMessage::V1(v1) = &message else {
        return Err(bad("expected v1 message"));
    };
    if v1.instructions.len() != 2 || v1.instructions[1].data.first() != Some(&tag::MERGE_TRANSACT) {
        return Err(bad("expected exactly CreateCache and MergeTransact"));
    }
    let data = MergeTransactIxData::deserialize(&v1.instructions[1].data[1..])
        .map_err(|_| bad("invalid merge data"))?;
    if data.cache_slot != Some(slot)
        || data.expiry_unix_ts <= now() as u64
        || data.expiry_unix_ts > run.target.expires_at as u64
        || !(2..=51).contains(&data.nullifiers.len())
    {
        return Err(bad("merge does not match run slot, expiry or input limit"));
    }
    let fee = v1.config.priority_fee.unwrap_or(0);
    if fee > run.target.max_priority_fee_lamports {
        return Err(bad("priority fee exceeds run limit"));
    }
    let payer = Pubkey::from_str(&run.target.sponsor).map_err(|_| bad("invalid sponsor"))?;
    let owner = Pubkey::from_str(&run.owner).map_err(|_| bad("invalid owner"))?;
    let create = CreateCache {
        payer,
        data: CreateCacheData {
            write_authority: payer,
            nonce: run.target.nonce,
            tree_id: run.target.tree_id,
            expires_at: run.target.expires_at,
        },
    };
    let merge = MergeTransact {
        input_tree: pda::tree(run.target.tree_id),
        output_tree: pda::tree(run.target.tree_id),
        payer,
        user_record: user_record_pda(&owner).0,
        data,
        cache: Some(CacheWriteAccounts {
            cache: create.cache(),
            writer: payer,
        }),
    };
    let canonical = compile_message(
        &payer,
        &[create.instruction(), merge.instruction()],
        *message.recent_blockhash(),
        ComputeBudgetConfig::new(run.target.compute_unit_limit).with_priority_fee(fee),
    )
    .map_err(|_| bad("cannot compile merge"))?;
    if canonical.serialize() != bytes {
        return Err(bad("message differs from allowed sponsored transaction"));
    }
    Ok(message)
}

async fn relay(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<Relay>,
) -> Result<Json<Value>> {
    app.authorize(&headers)?;
    let bytes = STANDARD
        .decode(&request.message)
        .map_err(|_| bad("invalid message base64"))?;
    let submission = {
        let _lock = app.journal_lock.lock().await;
        let mut run = app.load(&id)?;
        if let Some(previous) = run.submissions.get(&request.slot) {
            if previous.message != request.message {
                return Err(Failure(
                    StatusCode::CONFLICT,
                    "slot already submitted; reconcile its signature",
                ));
            }
            previous.clone()
        } else {
            let message = validate(&run, request.slot, &bytes)?;
            if run.target.sponsor != app.sponsor.pubkey().to_string() {
                return Err(bad("run sponsor changed"));
            }
            let tx =
                sign_transaction(message, &[&app.sponsor]).map_err(|_| bad("cannot sign merge"))?;
            let submission = Submission {
                message: request.message,
                signature: tx.signatures[0].to_string(),
                transaction: STANDARD.encode(
                    wincode::serialize(&tx).map_err(|_| bad("cannot serialize transaction"))?,
                ),
            };
            run.submissions.insert(request.slot, submission.clone());
            app.save(&run)?; // Durable BEFORE the first network submission.
            submission
        }
    };
    // A retry submits identical signed bytes, never signs a replacement.
    let accepted = app.rpc("sendTransaction", json!([submission.transaction, {"encoding":"base64","skipPreflight":false,"maxRetries":0}])).await.is_ok();
    Ok(Json(
        json!({"signature":submission.signature, "accepted":accepted,
        "outcome": if accepted { "submitted" } else { "unknown" }}),
    ))
}

async fn status(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    app.authorize(&headers)?;
    let run = {
        let _lock = app.journal_lock.lock().await;
        app.load(&id)?
    };
    let signatures: Vec<_> = run.submissions.values().map(|s| &s.signature).collect();
    let statuses = app
        .rpc(
            "getSignatureStatuses",
            json!([signatures, {"searchTransactionHistory":true}]),
        )
        .await?;
    Ok(Json(json!({"run":run,"statuses":statuses})))
}

async fn close(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    app.authorize(&headers)?;
    let run = {
        let _lock = app.journal_lock.lock().await;
        app.load(&id)?
    };
    // Never invalidate an in-flight final payment. Example cleanup is after
    // expiry; funds remain spendable through the normal indexed tree path.
    if now() < run.target.expires_at {
        return Err(bad("cleanup is allowed only after cache expiry"));
    }
    let payer = app.sponsor.pubkey();
    if run.target.sponsor != payer.to_string() {
        return Err(bad("run sponsor changed"));
    }
    let block = app
        .rpc("getLatestBlockhash", json!([{"commitment":"confirmed"}]))
        .await?;
    let hash = block["value"]["blockhash"]
        .as_str()
        .ok_or_else(unavailable)?
        .parse()
        .map_err(|_| unavailable())?;
    let ix = CloseCache {
        cache: pda::cache(&payer, run.target.nonce).0,
        rent_recipient: payer,
        writer: None,
    }
    .instruction();
    let message = compile_message(&payer, &[ix], hash, ComputeBudgetConfig::new(200_000))
        .map_err(|_| bad("cannot build cleanup"))?;
    let tx = sign_transaction(message, &[&app.sponsor]).map_err(|_| bad("cannot sign cleanup"))?;
    let encoded =
        STANDARD.encode(wincode::serialize(&tx).map_err(|_| bad("cannot encode cleanup"))?);
    app.rpc(
        "sendTransaction",
        json!([encoded, {"encoding":"base64","skipPreflight":false}]),
    )
    .await?;
    Ok(Json(json!({"signature":tx.signatures[0].to_string()})))
}

async fn proxy(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Result<Response> {
    app.authorize(&headers)?;
    let route = uri
        .path()
        .strip_prefix("/v1/zolana")
        .ok_or_else(|| bad("invalid proxy route"))?;
    let attestation = method == Method::GET && route == "/tee/v1/attestation";
    let merge_route = |path: &str| {
        matches!(
            path,
            "/prove/merge_8_1" | "/prove/merge_24_1" | "/prove/merge_52_1"
        )
    };
    let merge = method == Method::POST && merge_route(route);
    let status = method == Method::GET && route.strip_suffix("/status").is_some_and(merge_route);
    if !attestation && !merge && !status {
        return Err(Failure(
            StatusCode::NOT_FOUND,
            "only attestation, merge proof and merge status routes are exposed",
        ));
    }
    let header_true = |name| headers.get(name).and_then(|v| v.to_str().ok()) == Some("true");
    if merge || status {
        if !headers.contains_key("zolana-tee") || !headers.contains_key("zolana-tee-enc") {
            return Err(bad("encrypted TEE request required"));
        }
        if merge
            && (headers.get("content-type").and_then(|v| v.to_str().ok())
                != Some("application/octet-stream")
                || body.is_empty()
                || !(header_true("x-sync") ^ header_true("x-async"))
                || (headers.contains_key("x-sync") && headers.contains_key("x-async")))
        {
            return Err(bad("encrypted merge requires exactly one delivery mode"));
        }
        if status {
            // Inspect without rebuilding the original query: TEE binds its
            // exact encoding as well as the method and relative route.
            let parsed = reqwest::Url::parse(&format!("https://proxy.invalid{uri}"))
                .map_err(|_| bad("invalid status query"))?;
            let query: Vec<_> = parsed.query_pairs().collect();
            if query.len() != 1
                || query[0].0 != "jobId"
                || query[0].1.is_empty()
                || !headers.contains_key("zolana-tee-ciphertext")
                || !body.is_empty()
                || headers.contains_key("x-sync")
                || headers.contains_key("x-async")
            {
                return Err(bad("encrypted status request with one jobId required"));
            }
        }
    }
    let _permit = app
        .concurrency
        .try_acquire()
        .map_err(|_| Failure(StatusCode::TOO_MANY_REQUESTS, "example proxy is busy"))?;
    let mut upstream = app.prover.clone();
    upstream.set_path(&format!(
        "{}{}",
        app.prover.path().trim_end_matches('/'),
        route
    ));
    upstream.set_query(uri.query());
    let mut request = app.http.request(method, upstream);
    if status {
        request = request.timeout(Duration::from_secs(30));
    }
    for name in [
        "content-type",
        "x-sync",
        "x-async",
        "zolana-tee",
        "zolana-tee-enc",
        "zolana-tee-ciphertext",
    ] {
        if let Some(value) = headers.get(name) {
            request = request.header(name, value);
        }
    }
    // Optional upstream credentials stay server-side and do not alter the
    // method/path/query authenticated by TEE encryption.
    if let Ok(token) = std::env::var("PROVER_BEARER_TOKEN") {
        request = request.bearer_auth(token);
    }
    let mut answer = request.body(body).send().await.map_err(|_| unavailable())?;
    let status = answer.status();
    let response_headers = answer.headers().clone();
    let mut bytes = Vec::new();
    while let Some(chunk) = answer.chunk().await.map_err(|_| unavailable())? {
        if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
            return Err(unavailable());
        }
        bytes.extend_from_slice(&chunk);
    }
    let mut response = (status, bytes).into_response();
    for name in [
        "content-type",
        "zolana-tee",
        "zolana-tee-enc",
        "zolana-tee-ciphertext",
        "retry-after",
    ] {
        if let Some(value) = response_headers.get(name) {
            response.headers_mut().insert(name, value.clone());
        }
    }
    Ok(response)
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("SF_AUTH_TOKEN")?;
    if token.len() < 16 {
        return Err("SF_AUTH_TOKEN must contain at least 16 characters".into());
    }
    let key: Vec<u8> = serde_json::from_slice(&std::fs::read(std::env::var("SPONSOR_KEYPAIR")?)?)?;
    let sponsor = Keypair::try_from(key.as_slice())?;
    let prover = reqwest::Url::parse(&std::env::var("PROVER_URL")?)?;
    if prover.scheme() != "https"
        || prover.query().is_some()
        || prover.fragment().is_some()
        || !prover.username().is_empty()
        || prover.password().is_some()
    {
        return Err("PROVER_URL must be HTTPS without credentials, query or fragment".into());
    }
    let journal =
        PathBuf::from(std::env::var("RUN_DIR").unwrap_or_else(|_| ".example-runs".into()));
    std::fs::create_dir_all(&journal)?;
    let app = Arc::new(App {
        token,
        sponsor,
        prover,
        rpc_url: std::env::var("RPC_URL")?,
        journal,
        journal_lock: Mutex::new(()),
        concurrency: Semaphore::new(4),
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(600))
            .build()?,
    });
    let routes = Router::new()
        .route("/runs", post(start))
        .route("/runs/{id}", get(status))
        .route("/runs/{id}/merges", post(relay))
        .route("/runs/{id}/close", post(close))
        .route("/v1/zolana/{*path}", get(proxy).post(proxy))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(app);
    let address = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8788".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("Example proxy/relayer listening on {address}");
    axum::serve(listener, routes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zolana_interface::instruction::instruction_data::merge_transact::{
        MergeOutputEncryption, MergeProof,
    };

    fn fixture() -> (Run, VersionedMessage) {
        let payer = Keypair::new().pubkey();
        let owner = Keypair::new().pubkey();
        let run = Run {
            id: "0000000000000001".into(),
            owner: owner.to_string(),
            slots: 2,
            target: CacheTarget {
                sponsor: payer.to_string(),
                nonce: 1,
                tree_id: 0,
                expires_at: now() + 600,
                slot: 0,
                max_priority_fee_lamports: 5_000,
                compute_unit_limit: 400_000,
            },
            submissions: BTreeMap::new(),
        };
        let create = CreateCache {
            payer,
            data: CreateCacheData {
                write_authority: payer,
                nonce: 1,
                tree_id: 0,
                expires_at: run.target.expires_at,
            },
        };
        let merge = MergeTransact {
            input_tree: pda::tree(0),
            output_tree: pda::tree(0),
            payer,
            user_record: user_record_pda(&owner).0,
            cache: Some(CacheWriteAccounts {
                cache: create.cache(),
                writer: payer,
            }),
            data: MergeTransactIxData {
                expiry_unix_ts: (now() + 300) as u64,
                proof: MergeProof::zeroed(),
                output_utxo_hash: [1; 32],
                output_encryption: MergeOutputEncryption::PoseidonCtrV1 {
                    salt: [0; 16],
                    ciphertext: [[0; 32]; 3],
                },
                private_tx_hash: [0; 32],
                nullifiers: vec![[1; 32], [2; 32]],
                utxo_tree_root_index: 0,
                nullifier_tree_root_index: 0,
                cache_slot: Some(0),
            },
        };
        let message = compile_message(
            &payer,
            &[create.instruction(), merge.instruction()],
            Default::default(),
            ComputeBudgetConfig::new(400_000).with_priority_fee(5_000),
        )
        .unwrap();
        (run, message)
    }
    #[test]
    fn accepts_only_exact_run_transaction() {
        let (run, message) = fixture();
        assert!(validate(&run, 0, &message.serialize()).is_ok());
        assert!(validate(&run, 1, &message.serialize()).is_err());
        let mut other = run.clone();
        other.owner = Keypair::new().pubkey().to_string();
        assert!(validate(&other, 0, &message.serialize()).is_err());
        other = run.clone();
        other.target.nonce += 1;
        assert!(validate(&other, 0, &message.serialize()).is_err());
        for change in 0..4 {
            let mut altered = message.clone();
            let VersionedMessage::V1(v1) = &mut altered else {
                unreachable!()
            };
            match change {
                0 => v1.config.priority_fee = Some(5_001),
                1 => v1.instructions.push(v1.instructions[0].clone()),
                2 => v1.account_keys[0] = Keypair::new().pubkey(),
                _ => v1.config.compute_unit_limit = Some(1_400_000),
            }
            assert!(validate(&run, 0, &altered.serialize()).is_err());
        }
    }
    #[test]
    fn rejects_expired_or_malformed_runs_and_messages() {
        let (mut run, message) = fixture();
        run.target.expires_at = now() - 1;
        assert!(validate(&run, 0, &message.serialize()).is_err());
        assert!(validate(&run, 0, &[0; 40]).is_err());
        assert!(validate(&run, 0, &[0; 4096]).is_err());
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn queued_tee_jobs_preserve_encryption_delivery_and_exact_status_query() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use zolana_prover_tee::{seal_response, EncryptedRequest, FixtureProver};
        let fixture = Arc::new(FixtureProver::from_ikm(&[42; 32]));
        let key = fixture.hpke_public_key();
        let polls = Arc::new(AtomicUsize::new(0));
        let upstream = Router::new().route(
            "/{*path}",
            get({
                let fixture = fixture.clone();
                let polls = polls.clone();
                move |uri: Uri, headers: HeaderMap, body: Bytes| {
                    let fixture = fixture.clone();
                    let polls = polls.clone();
                    async move {
                        assert_eq!(uri.to_string(), "/prove/merge_24_1/status?jobId=job%2F1");
                        assert!(headers.get("x-sf-auth").is_none());
                        assert!(headers.get("x-sync").is_none());
                        assert!(headers.get("x-async").is_none());
                        assert!(body.is_empty());
                        let opened = fixture
                            .open(
                                "GET",
                                &uri.to_string(),
                                &hex::decode(headers["zolana-tee-enc"].to_str().unwrap()).unwrap(),
                                &hex::decode(headers["zolana-tee-ciphertext"].to_str().unwrap())
                                    .unwrap(),
                            )
                            .unwrap();
                        assert!(opened.plaintext.is_empty());
                        let response = match polls.fetch_add(1, Ordering::Relaxed) {
                            0 => json!({"status":"processing"}),
                            1 => json!({"status":"completed","result":{"opaqueProof":"test"}}),
                            _ => json!({"status":"failed","error":"test failure"}),
                        };
                        (
                            [("zolana-tee", "v1")],
                            seal_response(
                                &opened.response_key,
                                200,
                                &serde_json::to_vec(&response).unwrap(),
                            )
                            .unwrap(),
                        )
                    }
                }
            })
            .post(move |uri: Uri, headers: HeaderMap, body: Bytes| {
                let fixture = fixture.clone();
                async move {
                    assert_eq!(uri.path(), "/prove/merge_24_1");
                    assert_eq!(headers["x-async"], "true");
                    assert!(headers.get("x-sync").is_none());
                    assert!(headers.get("x-sf-auth").is_none());
                    let opened = fixture
                        .open(
                            "POST",
                            &uri.to_string(),
                            &hex::decode(headers["zolana-tee-enc"].to_str().unwrap()).unwrap(),
                            &body,
                        )
                        .unwrap();
                    assert_eq!(opened.plaintext, b"private witness");
                    (
                        StatusCode::ACCEPTED,
                        [("zolana-tee", "v1")],
                        seal_response(
                            &opened.response_key,
                            202,
                            br#"{"jobId":"job/1","status":"queued"}"#,
                        )
                        .unwrap(),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        let app = Arc::new(App {
            token: "token".into(),
            sponsor: Keypair::new(),
            prover: format!("http://{address}").parse().unwrap(),
            rpc_url: String::new(),
            http: reqwest::Client::new(),
            journal: PathBuf::new(),
            journal_lock: Mutex::new(()),
            concurrency: Semaphore::new(1),
        });
        for (method, route, expected) in [
            (Method::POST, "/prove/merge_24_1", "queued"),
            (
                Method::GET,
                "/prove/merge_24_1/status?jobId=job%2F1",
                "processing",
            ),
            (
                Method::GET,
                "/prove/merge_24_1/status?jobId=job%2F1",
                "completed",
            ),
            (
                Method::GET,
                "/prove/merge_24_1/status?jobId=job%2F1",
                "failed",
            ),
        ] {
            let post = method == Method::POST;
            let encrypted = EncryptedRequest::encrypt(
                &key,
                method.as_str(),
                route,
                if post { b"private witness" } else { b"" },
            )
            .unwrap();
            let mut headers = HeaderMap::new();
            for (name, value) in [
                ("x-sf-auth", "token"),
                ("zolana-tee", "v1"),
                ("zolana-tee-enc", &encrypted.enc),
            ] {
                headers.insert(name, value.parse().unwrap());
            }
            let body = if post {
                headers.insert("content-type", "application/octet-stream".parse().unwrap());
                headers.insert("x-async", "true".parse().unwrap());
                Bytes::from(encrypted.body.clone())
            } else {
                headers.insert(
                    "zolana-tee-ciphertext",
                    hex::encode(&encrypted.body).parse().unwrap(),
                );
                Bytes::new()
            };
            let uri: Uri = format!("/v1/zolana{route}").parse().unwrap();
            if !post {
                let mut missing_auth = headers.clone();
                missing_auth.remove("x-sf-auth");
                assert!(matches!(
                    proxy(
                        State(app.clone()),
                        missing_auth,
                        method.clone(),
                        uri.clone(),
                        body.clone()
                    )
                    .await,
                    Err(Failure(StatusCode::UNAUTHORIZED, _))
                ));
                let mut plaintext = headers.clone();
                plaintext.remove("zolana-tee-ciphertext");
                assert!(matches!(
                    proxy(
                        State(app.clone()),
                        plaintext,
                        method.clone(),
                        uri.clone(),
                        body.clone()
                    )
                    .await,
                    Err(Failure(StatusCode::BAD_REQUEST, _))
                ));
                assert!(matches!(
                    proxy(
                        State(app.clone()),
                        headers.clone(),
                        method.clone(),
                        "/v1/zolana/prove/merge_24_1/status?jobId=a&jobId=b"
                            .parse()
                            .unwrap(),
                        body.clone()
                    )
                    .await,
                    Err(Failure(StatusCode::BAD_REQUEST, _))
                ));
            }
            let response = proxy(State(app.clone()), headers, method, uri, body)
                .await
                .ok()
                .unwrap();
            assert_eq!(
                response.status(),
                if post {
                    StatusCode::ACCEPTED
                } else {
                    StatusCode::OK
                }
            );
            assert_eq!(response.headers()["zolana-tee"], "v1");
            let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
            let (status, plain) = encrypted.decrypt(&bytes).unwrap();
            assert_eq!(status, if post { 202 } else { 200 });
            let value: Value = serde_json::from_slice(&plain).unwrap();
            assert_eq!(value["status"], expected);
        }
        assert_eq!(polls.load(Ordering::Relaxed), 3);
        task.abort();
    }

    #[tokio::test]
    async fn proxy_auth_routes_and_encrypted_bytes_are_preserved() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = Router::new().route(
            "/prove/merge_8_1",
            post(|headers: HeaderMap, body: Bytes| async move {
                assert!(headers.get("x-sf-auth").is_none());
                assert_eq!(headers["x-sync"], "true");
                assert_eq!(headers["zolana-tee-enc"], "opaque-key");
                assert_eq!(body.as_ref(), &[1, 2, 3]);
                (
                    [
                        ("zolana-tee", "1"),
                        ("content-type", "application/octet-stream"),
                    ],
                    vec![4u8, 5, 6],
                )
            }),
        );
        let task = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        let app = Arc::new(App {
            token: "token".into(),
            sponsor: Keypair::new(),
            prover: format!("http://{address}").parse().unwrap(),
            rpc_url: String::new(),
            http: reqwest::Client::new(),
            journal: PathBuf::new(),
            journal_lock: Mutex::new(()),
            concurrency: Semaphore::new(1),
        });
        let mut headers = HeaderMap::new();
        for (name, value) in [
            ("x-sf-auth", "token"),
            ("x-sync", "true"),
            ("zolana-tee", "1"),
            ("zolana-tee-enc", "opaque-key"),
            ("content-type", "application/octet-stream"),
        ] {
            headers.insert(name, value.parse().unwrap());
        }
        let uri: Uri = "/v1/zolana/prove/merge_8_1".parse().unwrap();
        let response = proxy(
            State(app.clone()),
            headers.clone(),
            Method::POST,
            uri.clone(),
            Bytes::from_static(&[1, 2, 3]),
        )
        .await
        .ok()
        .unwrap();
        assert_eq!(response.headers()["zolana-tee"], "1");
        assert_eq!(
            to_bytes(response.into_body(), 100).await.unwrap().as_ref(),
            &[4, 5, 6]
        );
        headers.remove("x-sf-auth");
        assert!(matches!(
            proxy(
                State(app.clone()),
                headers.clone(),
                Method::POST,
                uri,
                Bytes::new()
            )
            .await,
            Err(Failure(StatusCode::UNAUTHORIZED, _))
        ));
        headers.insert("x-sf-auth", "token".parse().unwrap());
        assert!(matches!(
            proxy(
                State(app),
                headers,
                Method::GET,
                "/v1/zolana/prove/transfer_confidential_2_2/status"
                    .parse()
                    .unwrap(),
                Bytes::new()
            )
            .await,
            Err(Failure(StatusCode::NOT_FOUND, _))
        ));
        task.abort();
    }
}
