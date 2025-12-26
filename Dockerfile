
# Builder Stage
FROM rust:1.91-alpine AS builder

# install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

# Set WORKING Directory in the docker \
WORKDIR /app

# firstly need to copy the dependency files first for caching
COPY Cargo.toml Cargo.lock ./

# creating a dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build Dependencies only
RUN cargo build --release

# copy actual source code, we are copying from current src to the docker file system src
COPY src ./src

# Build actual Binary
RUN cargo build --release


# RUNTIME
FROM alpine:3.19

# Install runtime dependencies only
RUN apk add --no-cache \
    ca-certificates

# Create non-root user
RUN addgroup -S app && adduser -S app -G app

# Copy compiled binary from builder
COPY --from=builder /app/target/release/app /app/app

# Set permissions
RUN chown -R app:app /app

# Switch to non-root user
USER app

# Expose application port
EXPOSE 4545

# Runtime environment variables
ENV RUST_LOG=info
ENV APP_ENV=production

# Start the Axum server
CMD ["./app"]