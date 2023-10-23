FROM rust:bookworm as builder

WORKDIR /app/anti-entropy

COPY . /app/anti-entropy

RUN cargo test -- --test-threads=1
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR app

COPY --from=builder /app/anti-entropy/target/release/anti_entropy /app

EXPOSE 8080

ENV RUST_BACKTRACE=full
ENTRYPOINT ["/app/anti_entropy"]
