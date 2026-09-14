// === Entry point

package main

import (
	"log"
	"net/http"
)

func main() {
	config, err := LoadConfig()
	if err != nil {
		log.Fatalf("the service is misconfigured: %v", err)
	}

	address := config.Address()
	log.Printf("listening on http://%s", address)

	if err := http.ListenAndServe(address, NewRouter()); err != nil {
		log.Fatal(err)
	}
}
