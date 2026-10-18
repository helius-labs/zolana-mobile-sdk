package main

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"time"
	"unsafe"

	curve "github.com/consensys/gnark-crypto/ecc/bn254"
	"github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/backend/groth16"
	native "github.com/consensys/gnark/backend/groth16/bn254"
	"github.com/consensys/gnark/backend/witness"
	"zolana/prover/prover/common"
)

// The ringil prover takes the arithmetic: Go hands it the proving key's
// vectors and solver program once per key, and per proof the witness
// vector, then verifies and encodes the proof it assembled. Every slice is
// the memory of gnark's types, four little-endian Montgomery limbs per field
// element, which is the kernel's encoding.

type ringilInputs struct {
	a, b, k, z, b2, program, coefficients, fixed, domain []byte
}

// The memory of a slice of plain values, to send as it is.
func bytesOf[T any](values []T) []byte {
	if len(values) == 0 {
		return nil
	}
	var element T
	return unsafe.Slice((*byte)(unsafe.Pointer(&values[0])), int(unsafe.Sizeof(element))*len(values))
}

func ringilInputsOf(handle uint64) (ringilInputs, error) {
	prover, ok := prepared[handle]
	if !ok {
		return ringilInputs{}, errHandle
	}
	pk := prover.pk
	program := kernelSolverProgram(prover.cs, pk.InfinityA, pk.InfinityB)
	if program == nil {
		return ringilInputs{}, errUnsupported
	}
	fixed := make([]byte, 0, 448)
	fixed = append(fixed, bytesOf([]curve.G1Affine{pk.G1.Alpha, pk.G1.Beta, pk.G1.Delta})...)
	fixed = append(fixed, bytesOf([]curve.G2Affine{pk.G2.Beta, pk.G2.Delta})...)
	return ringilInputs{
		a:            bytesOf(pk.G1.A),
		b:            bytesOf(pk.G1.B),
		k:            bytesOf(pk.G1.K),
		z:            bytesOf(pk.G1.Z),
		b2:           bytesOf(pk.G2.B),
		program:      bytesOf(program),
		coefficients: bytesOf(prover.cs.Coefficients),
		fixed:        fixed,
		domain:       bytesOf([]fr.Element{pk.Domain.Generator, pk.Domain.FrMultiplicativeGen}),
	}, nil
}

// The witness vector of a request, public then secret, without the ONE
// wire; the public part stays with the prover for the proof's verification.
func ringilWitnessOf(handle uint64, request string) ([]byte, uint64, error) {
	prover, ok := prepared[handle]
	if !ok {
		return nil, 0, errHandle
	}
	started := time.Now()
	full, err := buildRequestWitness(request, prover.cs)
	if err != nil {
		return nil, 0, err
	}
	public, err := full.Public()
	if err != nil {
		return nil, 0, errBackend
	}
	prover.ringilPublic = public
	return bytesOf(full.Vector().(fr.Vector)), uint64(time.Since(started).Milliseconds()), nil
}

// Verifies and encodes a proof ringil assembled, Ar, Krs and Bs in gnark's
// raw encoding, against the last witness's public part.
func ringilFinish(handle uint64, raw []byte, proveMS uint64) (proofResult, error) {
	prover, ok := prepared[handle]
	if !ok {
		return proofResult{}, errHandle
	}
	if prover.ringilPublic == nil || len(raw) != 256 {
		return proofResult{}, errProof
	}
	var proof native.Proof
	for _, step := range []struct {
		point interface{ SetBytes([]byte) (int, error) }
		data  []byte
	}{{&proof.Ar, raw[:64]}, {&proof.Krs, raw[64:128]}, {&proof.Bs, raw[128:]}} {
		if _, err := step.point.SetBytes(step.data); err != nil {
			return proofResult{}, errProof
		}
	}
	public := prover.ringilPublic
	started := time.Now()
	if err := groth16.Verify(&proof, prover.vk, public); err != nil {
		return proofResult{}, errInvalidProof
	}
	verifyMS := uint64(time.Since(started).Milliseconds())
	var proofBytes bytes.Buffer
	if _, err := proof.WriteTo(&proofBytes); err != nil {
		return proofResult{}, errBackend
	}
	publicBytes, err := public.MarshalBinary()
	if err != nil {
		return proofResult{}, errBackend
	}
	canonical, err := json.Marshal(&common.Proof{Proof: &proof})
	if err != nil {
		return proofResult{}, errBackend
	}
	inputs, outputs, shapeKnown := circuitShape(prover.cs)
	return proofResult{
		proof:        hex.EncodeToString(proofBytes.Bytes()),
		publicInputs: hex.EncodeToString(publicBytes),
		proofJSON:    string(canonical),
		inputs:       inputs,
		outputs:      outputs,
		shapeKnown:   shapeKnown,
		proveMS:      proveMS,
		verifyMS:     verifyMS,
	}, nil
}

var _ witness.Witness = nil
