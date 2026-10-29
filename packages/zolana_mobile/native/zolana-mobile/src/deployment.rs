//! v0.6.0-alpha deployments. Policies come from the reviewed infra release
//! manifests, not from the prover being attested.

pub(crate) const DEVNET_POLICY: &str = include_str!("../deployments/devnet-e-tee-policy.json");
const MAINNET_POLICY: &str = include_str!("../deployments/mainnet-tee-policy.json");

/// Photon and Nitro endpoints with the trusted policy for that deployment.
/// Supply a Solana RPC on the same network separately in WalletConfig.
#[derive(Clone, Debug)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct Deployment {
    pub indexer_url: String,
    pub prover_url: String,
    pub prover_tee_policy: String,
}

impl Deployment {
    /// The devnet-e deployment on Solana devnet.
    #[flutter_rust_bridge::frb(sync)]
    pub fn devnet() -> Self {
        Self {
            indexer_url: "https://d34xe1hqb2ibg2.cloudfront.net".into(),
            prover_url: "https://dd73hh473h6ha.cloudfront.net".into(),
            prover_tee_policy: DEVNET_POLICY.into(),
        }
    }

    /// The v0.6.0-alpha deployment on Solana mainnet-beta.
    #[flutter_rust_bridge::frb(sync)]
    pub fn mainnet() -> Self {
        Self {
            indexer_url: "https://dgcj4pjarvo65.cloudfront.net".into(),
            prover_url: "https://d3ky2kt0ntqxt1.cloudfront.net".into(),
            prover_tee_policy: MAINNET_POLICY.into(),
        }
    }
}

pub(crate) fn policy_for_prover(url: &str) -> Option<&'static str> {
    let url = reqwest::Url::parse(url).ok()?;
    if url.scheme() != "https" || url.port_or_known_default() != Some(443) {
        return None;
    }
    match url.host_str()? {
        "dd73hh473h6ha.cloudfront.net" => Some(DEVNET_POLICY),
        "d3ky2kt0ntqxt1.cloudfront.net" => Some(MAINNET_POLICY),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zolana_client::prover::tee::TeePolicy;

    #[test]
    fn release_programs_and_deployment_policies_match() {
        assert_eq!(
            zolana_interface::PROGRAM_ID_PUBKEY.to_string(),
            "ring1GaLCuJnkUMVkX2JeZLTTX17wKsV4fZmPVzXANW"
        );
        assert_eq!(
            zolana_user_registry_interface::user_registry_program_id().to_string(),
            "regy3WAeRXMYuMYxr5iK3Bf9Arr3PcxGxyzAddbiqDQ"
        );
        for deployment in [Deployment::devnet(), Deployment::mainnet()] {
            TeePolicy::from_file_json(&deployment.prover_tee_policy).unwrap();
            assert_eq!(
                policy_for_prover(&format!("{}/v1/zolana", deployment.prover_url)),
                Some(deployment.prover_tee_policy.as_str())
            );
        }
        assert_ne!(DEVNET_POLICY, MAINNET_POLICY);
        assert_eq!(
            policy_for_prover("https://dd73hh473h6ha.cloudfront.net.attacker.test"),
            None
        );
        assert_eq!(
            policy_for_prover("http://dd73hh473h6ha.cloudfront.net"),
            None
        );
    }

    #[test]
    #[ignore = "live read-only attestation on mainnet and devnet-e"]
    fn live_deployment_attestations() {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        for deployment in [Deployment::devnet(), Deployment::mainnet()] {
            let policy = TeePolicy::from_file_json(&deployment.prover_tee_policy).unwrap();
            let prover = zolana_client::ProverClient::with_client(
                deployment.prover_url.clone(),
                http.clone(),
            )
            .with_tee(policy);
            prover.attest().unwrap();
            println!("{}: Nitro attestation verified", deployment.prover_url);
        }
    }
}
