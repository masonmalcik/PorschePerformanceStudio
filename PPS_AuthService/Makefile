.PHONY: build test lint lambda-build

GOOS := linux
GOARCH := amd64
CGO_ENABLED := 0
export GOOS GOARCH CGO_ENABLED

build:
	go build ./...

test:
	go test ./...

lint:
	go vet ./...

lambda-build:
	go build -trimpath -ldflags="-s -w" -o "$(ARTIFACTS_DIR)/bootstrap" ./cmd/lambda

build-AuthFunction: lambda-build
