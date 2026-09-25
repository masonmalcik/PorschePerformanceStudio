FROM golang:1.24-alpine AS build
WORKDIR /src
COPY go.mod go.sum* ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -trimpath -ldflags="-s -w" -o /out/auth-service ./cmd/api

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /out/auth-service /auth-service
EXPOSE 8082
USER nonroot:nonroot
ENTRYPOINT ["/auth-service"]

