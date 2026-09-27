FROM rust:1.88-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin local

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/local /usr/local/bin/pps-catalog
COPY assets ./assets
ENV PORT=3003 ASSET_ROOT=/app/assets RUST_LOG=info
EXPOSE 3003
USER 65532:65532
ENTRYPOINT ["pps-catalog"]
