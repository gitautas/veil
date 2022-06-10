package main

import (
	"fmt"
	"helgi/pkg/signalling"
	"os"
)

func main() {
	port := os.Getenv("HELGI_PORT")

	rpcServer, err := signalling.NewRPCServer()
	if err != nil {
		panic(err)
	}

	fmt.Println("Listening for gRPC calls on localhost:" + port)
	rpcServer.Serve(port)
}
