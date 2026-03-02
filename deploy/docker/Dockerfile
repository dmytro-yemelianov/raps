# RAPS multi-stage Docker build
# Shared builder compiles the raps binary once; per-service targets only differ in CMD.
#
# Build a specific service:
#   docker build --target worker -t rapscli/raps-worker .
#   docker build --target proxy  -t rapscli/raps-proxy  .
#
# Or use the service-specific Dockerfiles (thin wrappers around this file).

# ===========================================================================
# Stage 1: Builder — compile the raps binary with all features
# ===========================================================================
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf

WORKDIR /build
COPY . .

RUN cargo build --release --features kubernetes -p raps-cli \
    && strip target/release/raps

# ===========================================================================
# Stage 2: Runtime base — minimal Alpine with CA certs
# ===========================================================================
FROM alpine:3.21 AS runtime

RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/raps /usr/local/bin/raps

ENTRYPOINT ["raps"]

# ===========================================================================
# Per-service targets — differ only in the default CMD
# ===========================================================================

FROM runtime AS worker
CMD ["swarm", "worker", "start"]

FROM runtime AS proxy
CMD ["serve", "proxy"]

FROM runtime AS coordinator
CMD ["serve", "coordinator"]

FROM runtime AS webhook
CMD ["serve", "webhook"]

FROM runtime AS dashboard
CMD ["serve", "dashboard"]
