package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/require"
)

func Test_EpubToMarkdown(t *testing.T) {
	dir := "testdata"
	files, err := os.ReadDir("testdata")
	require.NoError(t, err)
	for _, file := range files {
		t.Run(file.Name(), func(t *testing.T) {
			require.NoError(t, EpubToMarkdown(filepath.Join(dir, file.Name())))
		})
	}
}
