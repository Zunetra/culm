// === Errors

package main

import (
	"encoding/json"
	"errors"
	"net/http"
)

// AppError is a failure carrying an HTTP status, so a handler reports it rather than assembling a response by hand.
type AppError struct {
	Status  int
	Message string
}

func (e *AppError) Error() string {
	return e.Message
}

// NotFound reports a path chi could not route to a handler.
func NotFound() *AppError {
	return &AppError{Status: http.StatusNotFound, Message: "not found"}
}

/* One body shape for every failure, so a client parses errors the same way
 * whatever produced them. An unexpected error is reported without its
 * message, because an internal detail is not a client's business. */
func writeError(w http.ResponseWriter, err error) {
	status := http.StatusInternalServerError
	message := "internal server error"

	var appErr *AppError
	if errors.As(err, &appErr) {
		status = appErr.Status
		message = appErr.Message
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(map[string]string{"error": message})
}
