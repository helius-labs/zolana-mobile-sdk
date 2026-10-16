//go:build !unix

package main

import "os"

// Without mmap, the image is read into memory: the same layout, loaded with
// one read instead of a decompression.
func mapFile(path string) ([]byte, error) {
	return os.ReadFile(path)
}

func unmapFile([]byte) {}
