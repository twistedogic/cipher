package main

import (
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/stretchr/testify/require"
)

func Test_FromURL(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		b, err := os.ReadFile("testdata/test.html")
		require.NoError(t, err)
		w.Write(b)
	}))
	defer ts.Close()
	got, err := FromURL(ts.URL)
	require.NoError(t, err)
	require.NotEmpty(t, got)
}
