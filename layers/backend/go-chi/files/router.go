// === The service

package main

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
)

/* NewRouter builds the router without binding a port. Separated from main so
 * an integration test drives the real routes in process, rather than a copy
 * of them that can drift. */
func NewRouter() http.Handler {
	r := chi.NewRouter()
	r.Get("/health", health)
	r.NotFound(notFound)
	return r
}

// health answers the liveness probe.
func health(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}

/* Registered rather than left to chi's default, so an unknown path fails in
 * the same shape as every other error. */
func notFound(w http.ResponseWriter, r *http.Request) {
	writeError(w, NotFound())
}
