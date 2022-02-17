package main

import (
	"fmt"
	"ymir/pkg"
)

func main() {
	ymir, err := ymir.NewYmir(4000, "localhost:3000")
	if err != nil {
		panic(err)
	}

	ymir.Serve()

	fmt.Println(ymir)
}
