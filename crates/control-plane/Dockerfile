# Сборка control-plane (volter-api). Образ строится из корня воркспейса:
#   docker build -f crates/control-plane/Dockerfile -t volter-api .
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --bin volter-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates git openssh-client \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/volter-api /usr/local/bin/volter-api
ENV VOLTER_BIND=0.0.0.0:8080 VOLTER_DATA_DIR=/data
EXPOSE 8080
CMD ["volter-api"]
