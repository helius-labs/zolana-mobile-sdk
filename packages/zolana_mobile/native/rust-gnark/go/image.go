package main

import (
	"bufio"
	"bytes"
	"encoding/binary"
	"errors"
	"os"
	"runtime"
	"unsafe"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark-crypto/ecc/bn254"
	"github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark-crypto/ecc/bn254/fr/pedersen"
	"github.com/consensys/gnark/backend/groth16"
	native "github.com/consensys/gnark/backend/groth16/bn254"
)

// A key image is the proving key of a `.key` container as this device holds
// it in memory, written beside the container after the first load. Reading
// the container decompresses every point of the key, seconds on a phone for
// a wide circuit; the image is mapped instead, and the system pages it in as
// the prover reads it and out again under memory pressure, where a key on
// the heap would take the app down with it.
//
// The layout is this device's. The header names the architecture, the byte
// order and the sizes of the types it holds, and the container it came from,
// and an image that does not match is ignored. Its points are not checked:
// the lockfile that vouched for the container vouches for them, as for a
// container read directly.

const (
	imageMagic   = "zolana-key-image"
	imageVersion = 1
	// Each slice starts on a cache line.
	imageAlign = 64
	// The slices before the commitment keys: G1 A, B, Z and K, G2 B,
	// InfinityA and InfinityB.
	imageFixedSections = 7
	// What fft.Domain.WriteTo writes.
	domainBytes = 8 + 5*fr.Bytes + 1
	// Bounds the section table of a malformed header.
	maxImageCommitments = 16
)

var errImage = errors.New("key image does not match")

// Its memory bytes tell the byte order of the device that wrote an image.
const imageOrder uint64 = 0x0102030405060708

// Encoded little-endian; the slices follow in the device's memory layout.
type imageHeader struct {
	Magic   [16]byte
	Version uint32
	_       uint32
	Arch    [16]byte
	Order   [8]byte
	SizeG1  uint32
	SizeG2  uint32
	SizeFr  uint32
	_       uint32
	// The container: its size, its twelve-byte header, and how long its
	// proving key section is, where the verifying key follows.
	KeySize   uint64
	KeyHeader keyHeader
	_         [4]byte
	PkLen     uint64
	// The proving key's counts and fixed points, as gnark writes them.
	NbWires       uint64
	NbInfinityA   uint64
	NbInfinityB   uint64
	NbCommitments uint32
	_             uint32
	Domain        [domainBytes + 7]byte
	Alpha         [bn254.SizeOfG1AffineUncompressed]byte
	Beta          [bn254.SizeOfG1AffineUncompressed]byte
	Delta         [bn254.SizeOfG1AffineUncompressed]byte
	Beta2         [bn254.SizeOfG2AffineUncompressed]byte
	Delta2        [bn254.SizeOfG2AffineUncompressed]byte
	// The file's length, sections included.
	Length uint64
}

// Where a slice lies in the file and how many elements it holds.
type imageSection struct {
	Count  uint64
	Offset uint64
}

func deviceOrder() (order [8]byte) {
	value := imageOrder
	copy(order[:], unsafe.Slice((*byte)(unsafe.Pointer(&value)), len(order)))
	return order
}

// The header this device writes, before the container and key fields.
func deviceImageHeader() imageHeader {
	var header imageHeader
	copy(header.Magic[:], imageMagic)
	header.Version = imageVersion
	copy(header.Arch[:], runtime.GOARCH)
	header.Order = deviceOrder()
	header.SizeG1 = uint32(unsafe.Sizeof(bn254.G1Affine{}))
	header.SizeG2 = uint32(unsafe.Sizeof(bn254.G2Affine{}))
	header.SizeFr = uint32(unsafe.Sizeof(fr.Element{}))
	return header
}

// Whether this device wrote `header`, from a container of `keySize` bytes
// with `keyHeader`.
func (header *imageHeader) matches(keySize uint64, keyHeader keyHeader) bool {
	device := deviceImageHeader()
	return header.Magic == device.Magic && header.Version == device.Version &&
		header.Arch == device.Arch && header.Order == device.Order &&
		header.SizeG1 == device.SizeG1 && header.SizeG2 == device.SizeG2 &&
		header.SizeFr == device.SizeFr &&
		header.KeySize == keySize && header.KeyHeader == keyHeader &&
		header.NbCommitments <= maxImageCommitments
}

// The memory of a slice of plain values.
func sliceBytes[T any](values []T) []byte {
	if len(values) == 0 {
		return nil
	}
	var element T
	return unsafe.Slice((*byte)(unsafe.Pointer(&values[0])), int(unsafe.Sizeof(element))*len(values))
}

type imagePart struct {
	count uint64
	bytes []byte
}

func part[T any](values []T) imagePart {
	return imagePart{count: uint64(len(values)), bytes: sliceBytes(values)}
}

// The slices of `pk` in the order of the image's sections.
func imageParts(pk *native.ProvingKey) []imagePart {
	parts := []imagePart{
		part(pk.G1.A), part(pk.G1.B), part(pk.G1.Z), part(pk.G1.K), part(pk.G2.B),
		part(pk.InfinityA), part(pk.InfinityB),
	}
	for _, key := range pk.CommitmentKeys {
		parts = append(parts, part(key.Basis), part(key.BasisExpSigma))
	}
	return parts
}

func aligned(offset uint64) uint64 {
	return (offset + imageAlign - 1) / imageAlign * imageAlign
}

// writeKeyImage writes the image of `pk`, read from the `pkLen`-byte proving
// key section of the container `key` with `header`, to `path`, through a
// partial file so that a cut write leaves no image.
func writeKeyImage(path string, key *os.File, header keyHeader, pk *native.ProvingKey, pkLen int64) error {
	stat, err := key.Stat()
	if err != nil {
		return err
	}
	if len(pk.CommitmentKeys) > maxImageCommitments {
		return errImage
	}
	image := deviceImageHeader()
	image.KeySize = uint64(stat.Size())
	image.KeyHeader = header
	image.PkLen = uint64(pkLen)
	image.NbWires = uint64(len(pk.InfinityA))
	image.NbInfinityA = pk.NbInfinityA
	image.NbInfinityB = pk.NbInfinityB
	image.NbCommitments = uint32(len(pk.CommitmentKeys))
	var domain bytes.Buffer
	if _, err := pk.Domain.WriteTo(&domain); err != nil || domain.Len() != domainBytes {
		return errImage
	}
	copy(image.Domain[:], domain.Bytes())
	image.Alpha = pk.G1.Alpha.RawBytes()
	image.Beta = pk.G1.Beta.RawBytes()
	image.Delta = pk.G1.Delta.RawBytes()
	image.Beta2 = pk.G2.Beta.RawBytes()
	image.Delta2 = pk.G2.Delta.RawBytes()

	parts := imageParts(pk)
	sections := make([]imageSection, len(parts))
	offset := aligned(uint64(binary.Size(image)) + uint64(binary.Size(sections)))
	for i, part := range parts {
		sections[i] = imageSection{Count: part.count, Offset: offset}
		offset = aligned(offset + uint64(len(part.bytes)))
	}
	image.Length = offset

	partial := path + ".partial"
	file, err := os.OpenFile(partial, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0o600)
	if err != nil {
		return err
	}
	err = func() error {
		writer := bufio.NewWriterSize(file, 1<<20)
		if err := binary.Write(writer, binary.LittleEndian, &image); err != nil {
			return err
		}
		if err := binary.Write(writer, binary.LittleEndian, sections); err != nil {
			return err
		}
		written := uint64(binary.Size(image)) + uint64(binary.Size(sections))
		for i, part := range parts {
			if _, err := writer.Write(make([]byte, sections[i].Offset-written)); err != nil {
				return err
			}
			if _, err := writer.Write(part.bytes); err != nil {
				return err
			}
			written = sections[i].Offset + uint64(len(part.bytes))
		}
		if _, err := writer.Write(make([]byte, image.Length-written)); err != nil {
			return err
		}
		if err := writer.Flush(); err != nil {
			return err
		}
		return file.Sync()
	}()
	if closeErr := file.Close(); err == nil {
		err = closeErr
	}
	if err == nil {
		err = os.Rename(partial, path)
	}
	if err != nil {
		_ = os.Remove(partial)
	}
	return err
}

// A proving key mapped from its image.
type keyImage struct {
	pk    *native.ProvingKey
	pkLen int64
	data  []byte
}

// openKeyImage maps the image at `path` of the container `key` with
// `header`. An image of another device or container, or a malformed one,
// fails with errImage.
func openKeyImage(path string, key *os.File, header keyHeader) (*keyImage, error) {
	stat, err := key.Stat()
	if err != nil {
		return nil, err
	}
	data, err := mapFile(path)
	if err != nil {
		return nil, err
	}
	image, err := readKeyImage(data, uint64(stat.Size()), header)
	if err != nil {
		unmapFile(data)
		return nil, err
	}
	image.data = data
	return image, nil
}

func readKeyImage(data []byte, keySize uint64, keyHeader keyHeader) (*keyImage, error) {
	reader := bytes.NewReader(data)
	var header imageHeader
	if err := binary.Read(reader, binary.LittleEndian, &header); err != nil ||
		!header.matches(keySize, keyHeader) || header.Length != uint64(len(data)) {
		return nil, errImage
	}
	sections := make([]imageSection, imageFixedSections+2*int(header.NbCommitments))
	if err := binary.Read(reader, binary.LittleEndian, sections); err != nil {
		return nil, errImage
	}
	pk := groth16.NewProvingKey(ecc.BN254).(*native.ProvingKey)
	if _, err := pk.Domain.ReadFrom(bytes.NewReader(header.Domain[:domainBytes])); err != nil {
		return nil, errImage
	}
	for point, raw := range map[*bn254.G1Affine][]byte{
		&pk.G1.Alpha: header.Alpha[:], &pk.G1.Beta: header.Beta[:], &pk.G1.Delta: header.Delta[:],
	} {
		if _, err := point.SetBytes(raw); err != nil {
			return nil, errImage
		}
	}
	for point, raw := range map[*bn254.G2Affine][]byte{&pk.G2.Beta: header.Beta2[:], &pk.G2.Delta: header.Delta2[:]} {
		if _, err := point.SetBytes(raw); err != nil {
			return nil, errImage
		}
	}
	pk.NbInfinityA, pk.NbInfinityB = header.NbInfinityA, header.NbInfinityB
	var err error
	for i, points := range []*[]bn254.G1Affine{&pk.G1.A, &pk.G1.B, &pk.G1.Z, &pk.G1.K} {
		if *points, err = view[bn254.G1Affine](data, sections[i]); err != nil {
			return nil, err
		}
	}
	if pk.G2.B, err = view[bn254.G2Affine](data, sections[4]); err != nil {
		return nil, err
	}
	if pk.InfinityA, err = flags(data, sections[5], header.NbWires); err != nil {
		return nil, err
	}
	if pk.InfinityB, err = flags(data, sections[6], header.NbWires); err != nil {
		return nil, err
	}
	pk.CommitmentKeys = make([]pedersen.ProvingKey, header.NbCommitments)
	for i := range pk.CommitmentKeys {
		key := &pk.CommitmentKeys[i]
		if key.Basis, err = view[bn254.G1Affine](data, sections[imageFixedSections+2*i]); err != nil {
			return nil, err
		}
		if key.BasisExpSigma, err = view[bn254.G1Affine](data, sections[imageFixedSections+2*i+1]); err != nil {
			return nil, err
		}
	}
	return &keyImage{pk: pk, pkLen: int64(header.PkLen)}, nil
}

// view is the slice of `section` in `data`, in place.
func view[T any](data []byte, section imageSection) ([]T, error) {
	var element T
	size := uint64(unsafe.Sizeof(element))
	if section.Offset%imageAlign != 0 || section.Offset > uint64(len(data)) ||
		section.Count > (uint64(len(data))-section.Offset)/size {
		return nil, errImage
	}
	if section.Count == 0 {
		return nil, nil
	}
	return unsafe.Slice((*T)(unsafe.Pointer(&data[section.Offset])), section.Count), nil
}

// flags copies the `count` infinity flags of `section`: a bool must be 0 or
// 1, which a mapped byte is not promised to be.
func flags(data []byte, section imageSection, count uint64) ([]bool, error) {
	raw, err := view[byte](data, section)
	if err != nil || uint64(len(raw)) != count {
		return nil, errImage
	}
	flags := make([]bool, len(raw))
	for i, value := range raw {
		if value > 1 {
			return nil, errImage
		}
		flags[i] = value == 1
	}
	return flags, nil
}

// close unmaps the image. The key's slices are cut first, so that a use
// after the close fails in Go rather than on the mapping.
func (image *keyImage) close() {
	pk := image.pk
	pk.G1.A, pk.G1.B, pk.G1.Z, pk.G1.K, pk.G2.B = nil, nil, nil, nil, nil
	pk.CommitmentKeys = nil
	unmapFile(image.data)
	image.data = nil
}
