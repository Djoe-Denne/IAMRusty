# Build stage
FROM rust:1.87-alpine AS builder

# Install dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    cmake \
    g++ \
    make \
    pkgconfig

# Set working directory
WORKDIR /usr/src/app

# Copy shared dependencies
COPY iam-events ./iam-events

# Copy rustycog workspace  
COPY rustycog ./rustycog

# Copy Telegraph workspace
COPY Telegraph ./Telegraph

# Build the application
WORKDIR /usr/src/app/Telegraph
RUN cargo build --release

# Runtime stage
FROM alpine:3.18

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    curl

# Create app user and directory
RUN adduser -D -s /bin/sh appuser
WORKDIR /app

# Copy the binary
COPY --from=builder /usr/src/app/Telegraph/target/release/telegraph-service /app/telegraph-service

# Copy configuration files
COPY --from=builder /usr/src/app/Telegraph/config /app/config

# Change ownership
RUN chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Set entrypoint
ENTRYPOINT ["./telegraph-service"] 