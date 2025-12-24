# Multi-stage build for IronFlow-Rs
FROM rust:1.70 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 ironflow

# Copy binary from builder
COPY --from=builder /app/target/release/ironflow-rs /usr/local/bin/ironflow-rs

# Set ownership
RUN chown ironflow:ironflow /usr/local/bin/ironflow-rs

# Switch to non-root user
USER ironflow

# Set working directory
WORKDIR /home/ironflow

# Expose health check port (if needed in future)
# EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the binary
CMD ["ironflow-rs"]
