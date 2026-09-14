// === Configuration

package main

import (
	"fmt"
	"os"
	"strconv"
)

/* Everything the service reads from the environment. Resolved once at
 * startup rather than at each use, so a misconfigured deployment fails
 * immediately instead of on the first request that happens to need a
 * value. */
type Config struct {
	Host string
	Port int
}

// LoadConfig reads the environment, falling back to values that work on a developer's machine.
func LoadConfig() (Config, error) {
	host := os.Getenv("HOST")
	if host == "" {
		host = "0.0.0.0"
	}

	portValue := os.Getenv("PORT")
	if portValue == "" {
		portValue = "3000"
	}

	port, err := strconv.Atoi(portValue)
	if err != nil || port < 0 || port > 65535 {
		return Config{}, fmt.Errorf("PORT is set to '%s', which is not a port number", portValue)
	}

	return Config{Host: host, Port: port}, nil
}

// Address renders the host and port as something net/http can listen on.
func (c Config) Address() string {
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}
