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
    /// can be checked against each other, and its points are taken as the
    /// lockfile vouches for them, without subgroup checks.
    ///
    /// With an `image` path, the proving key is mapped from the key image
    /// there when one matches the container, and written there after a load
    /// from the container otherwise: about twice the proving key's size on
    /// disk, for a load that takes no time and memory the system can page
    /// out. Remove the image with its container.
    pub fn load_key(key: &str, image: Option<&str>) -> Result<Self> {
        init()?;
        let key = input_string(key)?;
        let image = input_string(image.unwrap_or_default())?;
        let raw = unsafe {
            bind::gnark_prepared_load_key(key.as_ptr().cast_mut(), image.as_ptr().cast_mut())
        };
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

/// What the ringil prover takes from a loaded key: the five point vectors
/// and the solver program and coefficients in the kernel's encoding, the
/// proving key's fixed points (448 bytes) and the domain's generator and
/// coset generator (64 bytes). Borrowed from the bridge's buffers, which
/// live for the call of [`PreparedProver::with_ringil_inputs`]: a wide key
/// is hundreds of megabytes, and a phone has no room for another copy.
#[derive(Debug, Clone, Copy)]
pub struct RingilInputs<'a> {
    pub a: &'a [u8],
    pub b: &'a [u8],
    pub k: &'a [u8],
    pub z: &'a [u8],
    pub b2: &'a [u8],
    pub program: &'a [u8],
    pub coefficients: &'a [u8],
    pub fixed: &'a [u8],
    pub domain: &'a [u8],
}

struct RingilInputsGuard(*mut bind::C_RingilInputs);
impl Drop for RingilInputsGuard {
    fn drop(&mut self) {
        unsafe { bind::gnark_free_ringil_inputs(self.0) };
    }
}
struct RingilWitnessGuard(*mut bind::C_RingilWitness);
impl Drop for RingilWitnessGuard {
    fn drop(&mut self) {
        unsafe { bind::gnark_free_ringil_witness(self.0) };
    }
}

fn copy_bytes(bytes: &bind::C_Bytes) -> Vec<u8> {
    borrow_bytes(bytes).to_vec()
}

fn borrow_bytes(bytes: &bind::C_Bytes) -> &[u8] {
    if bytes.data.is_null() || bytes.len == 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(bytes.data, bytes.len as usize) }
}

impl PreparedProver {
    /// Runs `take` on the key's inputs for ringil, borrowed from the
    /// bridge's buffers, which are freed when it returns.
    pub fn with_ringil_inputs<R>(&self, take: impl FnOnce(RingilInputs<'_>) -> R) -> Result<R> {
        let raw = unsafe { bind::gnark_prepared_ringil_inputs(self.handle) };
        if raw.is_null() {
            bail!("gnark operation failed");
        }
        let guard = RingilInputsGuard(raw);
        let inputs = unsafe { &*guard.0 };
        if !inputs.error.is_null() {
            return Err(backend_error(inputs.error));
        }
        Ok(take(RingilInputs {
            a: borrow_bytes(&inputs.a),
            b: borrow_bytes(&inputs.b),
            k: borrow_bytes(&inputs.k),
            z: borrow_bytes(&inputs.z),
            b2: borrow_bytes(&inputs.b2),
            program: borrow_bytes(&inputs.program),
            coefficients: borrow_bytes(&inputs.coefficients),
            fixed: borrow_bytes(&inputs.fixed),
            domain: borrow_bytes(&inputs.domain),
        }))
    }

    /// Drops gnark's proving key vectors once ringil holds the key; the
    /// verifying key and constraint system stay for the witness and the
    /// verification. gnark can no longer prove with this handle.
    pub fn ringil_release_proving_key(&self) -> Result<()> {
        let error = unsafe { bind::gnark_prepared_ringil_release_pk(self.handle) };
        if error.is_null() {
            return Ok(());
        }
        let failure = backend_error(error);
        unsafe { bind::gnark_free_string(error) };
        Err(failure)
    }

    /// The request's witness vector (public then secret, no ONE wire) in
    /// the kernel's encoding, and how long building it took.
    pub fn ringil_witness(&self, request_json: &str) -> Result<(Vec<u8>, u64)> {
        let input = input_string(request_json)?;
        let raw =
            unsafe { bind::gnark_prepared_ringil_witness(self.handle, input.as_ptr().cast_mut()) };
        if raw.is_null() {
            bail!("gnark operation failed");
        }
        let guard = RingilWitnessGuard(raw);
        let witness = unsafe { &*guard.0 };
        if !witness.error.is_null() {
            return Err(backend_error(witness.error));
        }
        Ok((copy_bytes(&witness.witness), witness.witness_ms))
    }

    /// Verifies and encodes a proof ringil assembled from the last witness:
    /// Ar, Krs, Bs in gnark's raw encoding, 256 bytes.
    pub fn ringil_finish(&self, proof: &[u8], prove_ms: u64) -> Result<Groth16ProofResult> {
        let raw = unsafe {
            bind::gnark_prepared_ringil_finish(
                self.handle,
                proof.as_ptr().cast_mut(),
                proof.len() as u64,
                prove_ms,
            )
        };
        read_proof_result(raw)
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
