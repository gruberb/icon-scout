# Step 1: Build the Rust project using the Rust image
FROM rust:1.81 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
RUN cargo build --release

# Step 2: Use Ubuntu for the final image
FROM ubuntu:22.04
# Install necessary runtime libraries with explicit SSL setup
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
    ca-certificates \
    openssl \
    libssl-dev \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/icon-scout /usr/local/bin/icon-scout

# Set environment variables for SSL
ENV SSL_CERT_DIR=/etc/ssl/certs
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

EXPOSE 3000
CMD ["icon-scout"]
