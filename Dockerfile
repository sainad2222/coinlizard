FROM rust:1.84 as builder

WORKDIR /app

# Copy the manifests
COPY Cargo.toml .
COPY api/Cargo.toml api/
COPY common/Cargo.toml common/
COPY connectors/Cargo.toml connectors/
COPY store/Cargo.toml store/

# Create dummy files for all libraries
RUN mkdir -p api/src && echo "fn main() {}" > api/src/main.rs
RUN mkdir -p common/src && echo "pub fn dummy() {}" > common/src/lib.rs
RUN mkdir -p connectors/src && echo "pub fn dummy() {}" > connectors/src/lib.rs
RUN mkdir -p store/src && echo "pub fn dummy() {}" > store/src/lib.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy files
RUN rm -rf api/src common/src connectors/src store/src

# Copy the real source code
COPY api/src api/src
COPY common/src common/src
COPY connectors/src connectors/src
COPY store/src store/src

# Build the application
RUN cargo build --release

# Final stage - Using Ubuntu instead of Debian for newer glibc
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y ca-certificates curl strace && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/api /app/coinlizard-api

EXPOSE 3000

# Use a wrapper script to run the application with better error reporting
RUN echo '#!/bin/bash\nset -x\necho "Starting CoinLizard API"\necho "Environment: "\nenv\necho "Testing InfluxDB connection:"\ncurl -v ${INFLUXDB_URL} || true\necho "Running API with strace:"\nstrace -f -o /tmp/api.strace /app/coinlizard-api || (cat /tmp/api.strace && exit 1)' > /app/start.sh && \
    chmod +x /app/start.sh

CMD ["/app/start.sh"] 
