package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func Test_EpubToMarkdown(t *testing.T) {
	dir := "testdata"
	files, err := os.ReadDir("testdata")
	require.NoError(t, err)
	for _, file := range files {
		if !strings.HasSuffix(file.Name(), ".epub") {
			continue
		}
		t.Run(file.Name(), func(t *testing.T) {
			got, err := FromEpub(filepath.Join(dir, file.Name()))
			require.NoError(t, err)
			require.NotEmpty(t, got)
		})
	}
}
