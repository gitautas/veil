proto:
	protoc --proto_path=protobuf --go_out=protobuf/pb --go_opt=paths=source_relative --go-grpc_out=protobuf/pb --go-grpc_opt=paths=source_relative webrtc_signalling.proto
clean-all:
	go clean -i ./...
fmt:
	go fmt ./...
	goimports -w $(FILES)
test:
	go test -v ./... -short
start:
	docker compose up
