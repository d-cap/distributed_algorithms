FROM rust:bookworm as builder

WORKDIR /app/anti-entropy

COPY anti_entropy_server /app/anti-entropy
COPY merkle_tree /app/merkle_tree

RUN cargo test
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR app

COPY --from=builder /app/anti-entropy/target/release/anti_entropy /app

EXPOSE 8080

ENV RUST_BACKTRACE=full
ENTRYPOINT ["/app/anti_entropy"]
