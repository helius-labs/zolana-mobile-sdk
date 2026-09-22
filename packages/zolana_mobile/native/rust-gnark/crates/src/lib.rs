use anyhow::{anyhow, bail, Result};
use std::cell::Cell;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_char;

#[allow(warnings, clippy::all)]
mod bind {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[derive(Debug, Clone, Default)]
pub struct Groth16ProofResult {
    pub proof: String,
    pub public_inputs: String,
    pub proof_json: String,
    pub inputs: u32,
    pub outputs: u32,
    pub shape_known: bool,
    pub witness_ms: u64,
    pub prove_ms: u64,
    pub verify_ms: u64,
}

pub struct PreparedProver {
    handle: u64,
    not_sync: PhantomData<Cell<()>>,
}

impl PreparedProver {
    pub fn load(r1cs: &str, pk: &str, vk: &str) -> Result<Self> {
        init()?;
        let r1cs = input_string(r1cs)?;
        let pk = input_string(pk)?;
        let vk = input_string(vk)?;
        let raw = unsafe {
            bind::gnark_prepared_load(
                r1cs.as_ptr().cast_mut(),
                pk.as_ptr().cast_mut(),
                vk.as_ptr().cast_mut(),
            )
        };
        Self::from_result(raw)
    }

    /// Load a Zolana `.key` container (proving key, verifying key and
    /// constraint system in one file). Verify the file against the pinned
    /// proving-key lockfile first: the container is read before its sections
    /// can be checked against each other.
    pub fn load_key(key: &str) -> Result<Self> {
        init()?;
        let key = input_string(key)?;
        let raw = unsafe { bind::gnark_prepared_load_key(key.as_ptr().cast_mut()) };
        Self::from_result(raw)
    }

    fn from_result(raw: *mut bind::C_PreparedResult) -> Result<Self> {
        if raw.is_null() {
            bail!("gnark operation failed");
        }
        let output = PreparedResultGuard(raw);
        let result = unsafe { &*output.0 };
        if !result.error.is_null() {
            return Err(backend_error(result.error));
        }
        if result.handle == 0 {
            bail!("gnark operation failed");
        }
        Ok(Self {
            handle: result.handle,
            not_sync: PhantomData,
        })
    }

    pub fn prove(&self, witness_json: &str) -> Result<Groth16ProofResult> {
        let input = input_string(witness_json)?;
        let raw = unsafe { bind::gnark_prepared_prove(self.handle, input.as_ptr().cast_mut()) };
        read_proof_result(raw)
    }

    pub fn prove_request(&self, request_json: &str) -> Result<Groth16ProofResult> {
        let input = input_string(request_json)?;
        let raw =
            unsafe { bind::gnark_prepared_prove_request(self.handle, input.as_ptr().cast_mut()) };
        read_proof_result(raw)
    }

    pub fn verify(&self, result: &Groth16ProofResult) -> Result<bool> {
        let proof = input_string(&result.proof)?;
        let public_inputs = input_string(&result.public_inputs)?;
        let error = unsafe {
            bind::gnark_prepared_verify(
                self.handle,
                proof.as_ptr().cast_mut(),
                public_inputs.as_ptr().cast_mut(),
            )
        };
        read_verification(error)
    }
}

impl Drop for PreparedProver {
    fn drop(&mut self) {
        unsafe { bind::gnark_prepared_release(self.handle) };
    }
}

pub fn init() -> Result<()> {
    if unsafe { bind::gnark_init() } != 0 {
        bail!("gnark initialization failed");
    }
    Ok(())
}

pub fn groth16_prove(
    r1cs_path: &str,
    pk_path: &str,
    witness_json: &str,
) -> Result<Groth16ProofResult> {
    init()?;
    let r1cs = input_string(r1cs_path)?;
    let pk = input_string(pk_path)?;
    let witness = input_string(witness_json)?;
    let raw = unsafe {
        bind::gnark_groth16_prove(
            r1cs.as_ptr().cast_mut(),
            pk.as_ptr().cast_mut(),
            witness.as_ptr().cast_mut(),
        )
    };
    read_proof_result(raw)
}

pub fn groth16_verify(r1cs_path: &str, vk_path: &str, result: &Groth16ProofResult) -> Result<bool> {
    init()?;
    let r1cs = input_string(r1cs_path)?;
    let vk = input_string(vk_path)?;
    let proof = input_string(&result.proof)?;
    let public_inputs = input_string(&result.public_inputs)?;
    let error = unsafe {
        bind::gnark_groth16_verify(
            r1cs.as_ptr().cast_mut(),
            vk.as_ptr().cast_mut(),
            proof.as_ptr().cast_mut(),
            public_inputs.as_ptr().cast_mut(),
        )
    };
    read_verification(error)
}

fn input_string(input: &str) -> Result<CString> {
    CString::new(input).map_err(|_| anyhow!("invalid input encoding"))
}

fn backend_error(error: *const c_char) -> anyhow::Error {
    let message = unsafe { CStr::from_ptr(error) }.to_bytes();
    let fixed = match message {
        b"invalid proving system" => "invalid proving system",
        b"invalid witness" => "invalid witness",
        b"invalid proof request" => "invalid proof request",
        b"unsupported proof request" => "unsupported proof request",
        b"request does not match proving system" => "request does not match proving system",
        b"invalid proof encoding" => "invalid proof encoding",
        b"invalid proof" => "invalid proof",
        b"invalid prepared handle" => "invalid prepared handle",
        b"prepared prover capacity reached" => "prepared prover capacity reached",
        _ => "gnark operation failed",
    };
    anyhow!(fixed)
}

struct ProofResultGuard(*mut bind::C_Groth16ProofResult);

impl Drop for ProofResultGuard {
    fn drop(&mut self) {
        unsafe { bind::gnark_free_proof_result(self.0) };
    }
}

struct PreparedResultGuard(*mut bind::C_PreparedResult);

impl Drop for PreparedResultGuard {
    fn drop(&mut self) {
        unsafe { bind::gnark_free_prepared_result(self.0) };
    }
}

fn read_proof_result(raw: *mut bind::C_Groth16ProofResult) -> Result<Groth16ProofResult> {
    if raw.is_null() {
        bail!("gnark operation failed");
    }
    let output = ProofResultGuard(raw);
    let result = unsafe { &*output.0 };
    if !result.error.is_null() {
        return Err(backend_error(result.error));
    }
    if result.proof.is_null() || result.public_inputs.is_null() || result.proof_json.is_null() {
        bail!("gnark operation failed");
    }
    let copy_string = |value| unsafe {
        CStr::from_ptr(value)
            .to_str()
            .map(str::to_owned)
            .map_err(|_| anyhow!("gnark operation failed"))
    };
    Ok(Groth16ProofResult {
        proof: copy_string(result.proof)?,
        public_inputs: copy_string(result.public_inputs)?,
        proof_json: copy_string(result.proof_json)?,
        inputs: result.inputs,
        outputs: result.outputs,
        shape_known: result.shape_known != 0,
        witness_ms: result.witness_ms,
        prove_ms: result.prove_ms,
        verify_ms: result.verify_ms,
    })
}

fn read_verification(error: *mut c_char) -> Result<bool> {
    if error.is_null() {
        return Ok(true);
    }
    let failure = backend_error(error);
    unsafe { bind::gnark_free_string(error) };
    if failure.to_string() == "invalid proof" {
        Ok(false)
    } else {
        Err(failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_prover_is_send() {
        fn assert_send<Value: Send>() {}
        assert_send::<PreparedProver>();
    }

    #[test]
    fn nul_errors_do_not_include_input() {
        let error = input_string("PRIVATE_SENTINEL\0secret").unwrap_err();
        assert_eq!(format!("{error:?}"), "invalid input encoding");
        assert_eq!(format!("{error}"), "invalid input encoding");
    }

    #[test]
    fn unexpected_backend_errors_are_redacted() {
        let secret = CString::new("PRIVATE_SENTINEL").unwrap();
        assert_eq!(
            backend_error(secret.as_ptr()).to_string(),
            "gnark operation failed"
        );
    }

    #[test]
    fn missing_keys_have_fixed_errors() {
        let error = PreparedProver::load("PRIVATE_SENTINEL", "secret", "secret")
            .err()
            .unwrap();
        assert_eq!(error.to_string(), "invalid proving system");
    }
}
