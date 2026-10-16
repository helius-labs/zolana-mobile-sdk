//go:build unix

package main

import (
	"errors"
	"math"
	"os"
	"syscall"
)

// mapFile maps the whole file at `path` read-only. The system reads its
// pages when they are first used and may drop them under memory pressure,
// to read them again on the next use.
func mapFile(path string) ([]byte, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	stat, err := file.Stat()
	if err != nil {
		return nil, err
	}
	if stat.Size() <= 0 || stat.Size() > math.MaxInt32 {
		return nil, errors.New("key image size out of range")
	}
	return syscall.Mmap(int(file.Fd()), 0, int(stat.Size()), syscall.PROT_READ, syscall.MAP_PRIVATE)
}

func unmapFile(data []byte) {
	if data != nil {
		_ = syscall.Munmap(data)
	}
}
