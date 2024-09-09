# Step 1: Build the Rust project using the Rust image
FROM rust:1.81 AS builder

WORKDIR /app

# Copy the manifest and source code
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src

# Build the Rust application
RUN cargo build --release

# Step 2: Use Ubuntu for the final image
FROM ubuntu:22.04

# Install necessary runtime libraries
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from the previous stage
COPY --from=builder /app/target/release/icon-scout /usr/local/bin/icon-scout

# Expose the necessary port
EXPOSE 3000

# Run the application
CMD ["icon-scout"]
