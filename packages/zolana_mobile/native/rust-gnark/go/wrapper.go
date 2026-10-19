package main

/*
#include <stdint.h>
#include <stdlib.h>

typedef struct {
    char *proof;
    char *public_inputs;
    char *proof_json;
    uint32_t inputs;
    uint32_t outputs;
    uint32_t shape_known;
    uint64_t witness_ms;
    uint64_t prove_ms;
    uint64_t verify_ms;
    char *error;
} C_Groth16ProofResult;

typedef struct {
    uint64_t handle;
    char *error;
} C_PreparedResult;

typedef struct {
    uint8_t *data;
    uint64_t len;
} C_Bytes;

typedef struct {
    C_Bytes a, b, k, z, b2, program, coefficients, fixed, domain;
    char *error;
} C_ZoproInputs;

typedef struct {
    C_Bytes witness;
    uint64_t witness_ms;
    char *error;
} C_ZoproWitness;
*/
import "C"

import (
	"os"
	"runtime"
	"unsafe"
)

func cError(err error) *C.char {
	if err == nil {
		return nil
	}
	switch err {
	case errKey, errWitness, errRequest, errUnsupported, errShape, errProof, errInvalidProof, errHandle, errCapacity:
		return C.CString(err.Error())
	default:
		return C.CString(errBackend.Error())
	}
}

func exportProof(result proofResult, err error) *C.C_Groth16ProofResult {
	output := (*C.C_Groth16ProofResult)(C.calloc(1, C.size_t(unsafe.Sizeof(C.C_Groth16ProofResult{}))))
	if output == nil {
		return nil
	}
	if err != nil {
		output.error = cError(err)
		return output
	}
	output.proof = C.CString(result.proof)
	output.public_inputs = C.CString(result.publicInputs)
	output.proof_json = C.CString(result.proofJSON)
	output.inputs = C.uint32_t(result.inputs)
	output.outputs = C.uint32_t(result.outputs)
	if result.shapeKnown {
		output.shape_known = 1
	}
	output.witness_ms = C.uint64_t(result.witnessMS)
	output.prove_ms = C.uint64_t(result.proveMS)
	output.verify_ms = C.uint64_t(result.verifyMS)
	return output
}

func recoverProof(output **C.C_Groth16ProofResult) {
	if recover() != nil {
		*output = exportProof(proofResult{}, errBackend)
	}
}

func recoverError(output **C.char) {
	if recover() != nil {
		*output = cError(errBackend)
	}
}

//export gnark_init
func gnark_init() (status C.int) {
	defer func() {
		if recover() != nil {
			status = -1
		}
	}()
	if runtime.GOOS == "ios" || runtime.GOOS == "darwin" {
		_ = os.Setenv("GODEBUG", "asyncpreemptoff=1")
	}
	return 0
}

//export gnark_prepared_load
func gnark_prepared_load(r1csPath, pkPath, vkPath *C.char) (output *C.C_PreparedResult) {
	output = (*C.C_PreparedResult)(C.calloc(1, C.size_t(unsafe.Sizeof(C.C_PreparedResult{}))))
	if output == nil {
		return nil
	}
	defer func() {
		if recover() != nil {
			output.error = cError(errBackend)
		}
	}()
	handle, err := loadPrepared(C.GoString(r1csPath), C.GoString(pkPath), C.GoString(vkPath))
	output.handle = C.uint64_t(handle)
	output.error = cError(err)
	return output
}

//export gnark_prepared_load_key
func gnark_prepared_load_key(keyPath, imagePath *C.char) (output *C.C_PreparedResult) {
	output = (*C.C_PreparedResult)(C.calloc(1, C.size_t(unsafe.Sizeof(C.C_PreparedResult{}))))
	if output == nil {
		return nil
	}
	defer func() {
		if recover() != nil {
			output.error = cError(errBackend)
		}
	}()
	handle, err := loadPreparedKey(C.GoString(keyPath), C.GoString(imagePath))
	output.handle = C.uint64_t(handle)
	output.error = cError(err)
	return output
}

//export gnark_prepared_prove_request
func gnark_prepared_prove_request(handle C.uint64_t, input *C.char) (output *C.C_Groth16ProofResult) {
	defer recoverProof(&output)
	result, err := provePrepared(uint64(handle), C.GoString(input))
	return exportProof(result, err)
}

//export gnark_prepared_verify
func gnark_prepared_verify(handle C.uint64_t, proof, publicInputs *C.char) (output *C.char) {
	defer recoverError(&output)
	valid, err := verifyPrepared(uint64(handle), C.GoString(proof), C.GoString(publicInputs))
	if err == nil && !valid {
		err = errInvalidProof
	}
	return cError(err)
}

//export gnark_prepared_release
func gnark_prepared_release(handle C.uint64_t) {
	defer func() { _ = recover() }()
	releasePrepared(uint64(handle))
}

//export gnark_free_prepared_result
func gnark_free_prepared_result(result *C.C_PreparedResult) {
	if result != nil {
		C.free(unsafe.Pointer(result.error))
		C.free(unsafe.Pointer(result))
	}
}

//export gnark_free_proof_result
func gnark_free_proof_result(result *C.C_Groth16ProofResult) {
	if result != nil {
		C.free(unsafe.Pointer(result.proof))
		C.free(unsafe.Pointer(result.public_inputs))
		C.free(unsafe.Pointer(result.proof_json))
		C.free(unsafe.Pointer(result.error))
		C.free(unsafe.Pointer(result))
	}
}

//export gnark_free_string
func gnark_free_string(value *C.char) {
	C.free(unsafe.Pointer(value))
}

func main() {}

func cBytes(data []byte) C.C_Bytes {
	if len(data) == 0 {
		return C.C_Bytes{}
	}
	return C.C_Bytes{data: (*C.uint8_t)(C.CBytes(data)), len: C.uint64_t(len(data))}
}

func freeBytes(bytes C.C_Bytes) {
	if bytes.data != nil {
		C.free(unsafe.Pointer(bytes.data))
	}
}

//export gnark_prepared_zopro_inputs
func gnark_prepared_zopro_inputs(handle C.uint64_t) (output *C.C_ZoproInputs) {
	output = (*C.C_ZoproInputs)(C.calloc(1, C.size_t(unsafe.Sizeof(C.C_ZoproInputs{}))))
	if output == nil {
		return nil
	}
	defer func() {
		if recover() != nil {
			output.error = cError(errBackend)
		}
	}()
	inputs, err := backendCall(func() (zoproInputs, error) { return zoproInputsOf(uint64(handle)) })
	if err != nil {
		output.error = cError(err)
		return output
	}
	output.a = cBytes(inputs.a)
	output.b = cBytes(inputs.b)
	output.k = cBytes(inputs.k)
	output.z = cBytes(inputs.z)
	output.b2 = cBytes(inputs.b2)
	output.program = cBytes(inputs.program)
	output.coefficients = cBytes(inputs.coefficients)
	output.fixed = cBytes(inputs.fixed)
	output.domain = cBytes(inputs.domain)
	return output
}

//export gnark_free_zopro_inputs
func gnark_free_zopro_inputs(inputs *C.C_ZoproInputs) {
	if inputs == nil {
		return
	}
	for _, bytes := range []C.C_Bytes{inputs.a, inputs.b, inputs.k, inputs.z, inputs.b2, inputs.program, inputs.coefficients, inputs.fixed, inputs.domain} {
		freeBytes(bytes)
	}
	if inputs.error != nil {
		C.free(unsafe.Pointer(inputs.error))
	}
	C.free(unsafe.Pointer(inputs))
}

//export gnark_prepared_zopro_witness
func gnark_prepared_zopro_witness(handle C.uint64_t, input *C.char) (output *C.C_ZoproWitness) {
	output = (*C.C_ZoproWitness)(C.calloc(1, C.size_t(unsafe.Sizeof(C.C_ZoproWitness{}))))
	if output == nil {
		return nil
	}
	defer func() {
		if recover() != nil {
			output.error = cError(errBackend)
		}
	}()
	type built struct {
		vector []byte
		ms     uint64
	}
	result, err := backendCall(func() (built, error) {
		vector, ms, err := zoproWitnessOf(uint64(handle), C.GoString(input))
		return built{vector, ms}, err
	})
	if err != nil {
		output.error = cError(err)
		return output
	}
	output.witness = cBytes(result.vector)
	output.witness_ms = C.uint64_t(result.ms)
	return output
}

//export gnark_free_zopro_witness
func gnark_free_zopro_witness(w *C.C_ZoproWitness) {
	if w == nil {
		return
	}
	freeBytes(w.witness)
	if w.error != nil {
		C.free(unsafe.Pointer(w.error))
	}
	C.free(unsafe.Pointer(w))
}

//export gnark_prepared_zopro_release_pk
func gnark_prepared_zopro_release_pk(handle C.uint64_t) (output *C.char) {
	defer recoverError(&output)
	_, err := backendCall(func() (struct{}, error) { return struct{}{}, zoproReleaseProvingKey(uint64(handle)) })
	return cError(err)
}

//export gnark_prepared_zopro_finish
func gnark_prepared_zopro_finish(handle C.uint64_t, proof *C.uint8_t, length C.uint64_t, proveMS C.uint64_t) (output *C.C_Groth16ProofResult) {
	defer recoverProof(&output)
	raw := C.GoBytes(unsafe.Pointer(proof), C.int(length))
	return exportProof(backendCall(func() (proofResult, error) {
		return zoproFinish(uint64(handle), raw, uint64(proveMS))
	}))
}
