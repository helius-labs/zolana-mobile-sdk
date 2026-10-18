package main

import (
	"math"

	"github.com/consensys/gnark/constraint"
	cs "github.com/consensys/gnark/constraint/bn254"
	"github.com/consensys/gnark/constraint/solver"
)

// The hints the kernel solves itself, by the kind its plan names them with.
// Only gnark's own functions qualify: the fast path runs with gnark's hint
// registry, never with hints a caller passes.
var kernelHints = map[string]uint32{
	solver.GetHintName(solver.InvZeroHint):           0,
	"github.com/consensys/gnark/std/math/bits.nBits": 1,
}

// gnark 0.15 represents GKR work through custom blueprints/hints, which
// are rejected below along with every non-generic instruction.
// Export ordinary R1Cs and the hints of kernelHints, in gnark's dependency
// order. Other hints, custom blueprints, commitments, GKR and Println retain
// the original Go solver. Rust validates the dimensions, topology and
// coefficient table before use. The versioned bridge is internal to the
// generated Go/Rust pair.
func kernelSolverProgram(r1cs *cs.R1CS, infinityA, infinityB []bool) []uint32 {
	commitments, ok := r1cs.CommitmentInfo.(constraint.Groth16Commitments)
	if !ok || len(commitments) != 0 || len(r1cs.Logs) != 0 || r1cs.Type != constraint.SystemR1CS {
		return nil
	}
	hints := make(map[solver.HintID]uint32, len(r1cs.MHintsDependencies))
	for id, name := range r1cs.MHintsDependencies {
		kind, ok := kernelHints[name]
		if !ok {
			return nil
		}
		hints[id] = kind
	}
	nInputs := len(r1cs.Public) + len(r1cs.Secret)
	nWires := nInputs + r1cs.NbInternalVariables
	if len(r1cs.Public) == 0 || len(infinityA) != nWires || len(infinityB) != nWires {
		return nil
	}
	nHints := 0
	for _, instruction := range r1cs.Instructions {
		if int(instruction.BlueprintID) >= len(r1cs.Blueprints) {
			return nil
		}
		switch r1cs.Blueprints[instruction.BlueprintID].(type) {
		case *constraint.BlueprintGenericR1C:
		case *constraint.BlueprintGenericHint:
			nHints++
		default:
			return nil
		}
	}
	if len(r1cs.Instructions) != r1cs.GetNbConstraints()+nHints {
		return nil
	}
	indices := func(infinity []bool) []uint32 {
		out := make([]uint32, 0, len(infinity))
		for i, isInfinity := range infinity {
			if !isInfinity {
				out = append(out, uint32(i))
			}
		}
		return out
	}
	a, b := indices(infinityA), indices(infinityB)
	program := []uint32{2, uint32(nWires), uint32(nInputs), uint32(len(r1cs.Public)),
		uint32(r1cs.GetNbConstraints()), uint32(nHints), uint32(len(a)), uint32(len(b))}
	program = append(program, a...)
	program = append(program, b...)
	var r constraint.R1C
	var h constraint.HintMapping
	count := 0
	for _, level := range r1cs.Levels {
		for _, id := range level {
			if int(id) >= len(r1cs.Instructions) {
				return nil
			}
			instruction := r1cs.Instructions[id]
			if hint, ok := r1cs.Blueprints[instruction.BlueprintID].(*constraint.BlueprintGenericHint); ok {
				hint.DecompressHint(&h, instruction.Unpack(&r1cs.System))
				kind, ok := hints[h.HintID]
				if !ok || len(h.Inputs) != 1 {
					return nil
				}
				program = append(program, math.MaxUint32, kind, 1, uint32(len(h.Inputs[0])))
				for _, term := range h.Inputs[0] {
					program = append(program, term.CID, term.VID)
				}
				program = append(program, h.OutputRange.Start, h.OutputRange.End)
				continue
			}
			blueprint := r1cs.Blueprints[instruction.BlueprintID].(*constraint.BlueprintGenericR1C)
			blueprint.DecompressR1C(&r, instruction.Unpack(&r1cs.System))
			program = append(program, instruction.ConstraintOffset, uint32(len(r.L)), uint32(len(r.R)), uint32(len(r.O)))
			for _, expression := range []constraint.LinearExpression{r.L, r.R, r.O} {
				for _, term := range expression {
					program = append(program, term.CID, term.VID)
				}
			}
			count++
		}
	}
	if count != r1cs.GetNbConstraints() {
		return nil
	}
	return program
}
