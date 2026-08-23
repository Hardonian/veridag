FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/veridag
COPY . .

RUN cd implementations/rust && cargo build --release --bin veridag-node -p veridag-node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/veridag/implementations/rust/target/release/veridag-node /usr/local/bin/veridag-node

ENTRYPOINT ["veridag-node"]
