NAME = ymir

.PHONY: proto
proto:
	protoc --proto_path=../protobuf --go_out=./pkg/pb --go-grpc_out=./pkg/pb --go_opt=paths=source_relative --go-grpc_opt=paths=source_relative ../protobuf/*.proto

.PHONY: clean
clean:
	go mod tidy
	go clean -i ./...

.PHONY: lint
lint:
	go fmt ./...
	goimports -w $(FILES)

.PHONY: test
test:
	go test -v ./... # TODO: Add coverage checks

.PHONY: start
start:
	go run ./cmd/$(NAME).go

.PHONY: build
build:
	make proto
	go build ./cmd/$(NAME).go
