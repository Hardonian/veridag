FROM rust:1.95-bookworm AS builder

WORKDIR /usr/src/veridag
COPY . .

RUN cargo build --release --locked --bin veridag-node -p veridag-node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl \
    && groupadd --system veridag \
    && useradd --system --gid veridag --home-dir /var/lib/veridag veridag \
    && mkdir -p /var/lib/veridag \
    && chown veridag:veridag /var/lib/veridag \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder --chown=veridag:veridag /usr/src/veridag/target/release/veridag-node /usr/local/bin/veridag-node

USER veridag
WORKDIR /var/lib/veridag

HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=5 \
  CMD curl -sf http://localhost:8080/v1/health || exit 1

ENTRYPOINT ["veridag-node"]
