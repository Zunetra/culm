// === The health endpoint, over the real router

package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHealthReportsOk(t *testing.T) {
	request := httptest.NewRequest(http.MethodGet, "/health", nil)
	recorder := httptest.NewRecorder()

	NewRouter().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusOK {
		t.Fatalf("expected status %d, got %d", http.StatusOK, recorder.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(recorder.Body).Decode(&body); err != nil {
		t.Fatalf("the body did not decode as json: %v", err)
	}
	if body["status"] != "ok" {
		t.Fatalf("expected status \"ok\", got %q", body["status"])
	}
}

/* The fallback exists so a wrong path fails in the same shape as every other
 * error, rather than with an empty framework default. */
func TestAnUnknownPathReturnsTheSharedErrorBody(t *testing.T) {
	request := httptest.NewRequest(http.MethodGet, "/nope", nil)
	recorder := httptest.NewRecorder()

	NewRouter().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusNotFound {
		t.Fatalf("expected status %d, got %d", http.StatusNotFound, recorder.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(recorder.Body).Decode(&body); err != nil {
		t.Fatalf("the body did not decode as json: %v", err)
	}
	if body["error"] != "not found" {
		t.Fatalf("expected error \"not found\", got %q", body["error"])
	}
}
