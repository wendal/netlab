# ---- Build stage ----
FROM rust:1.75-slim AS builder

WORKDIR /build
COPY netlab-server/ .
RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/netlab-server /usr/local/bin/
COPY netlab-server/config/ /etc/netlab/config/
COPY netlab-server/cert/ /etc/netlab/cert/

WORKDIR /etc/netlab

EXPOSE 9073 9400 21000-29000

CMD ["netlab-server"]
