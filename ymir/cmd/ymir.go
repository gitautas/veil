package main

import (
	"fmt"
	"os"
	"ymir/pkg"
)

func main() {
	port := os.Getenv("YMIR_PORT")
	helgiPort := os.Getenv("HELGI_PORT")
	ymir, err := ymir.NewYmir(port, "localhost:" + helgiPort)
	if err != nil {
		panic(err)
	}

	ymir.Serve()

	fmt.Println(ymir)
}
