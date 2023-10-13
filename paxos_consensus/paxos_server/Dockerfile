FROM rust:bookworm as builder

WORKDIR /app/consensus

COPY . /app/consensus

#RUN cargo test -- --test-threads=1 
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR app

COPY --from=builder /app/consensus/target/release/paxos_server /app

EXPOSE 8080

ENV RUST_BACKTRACE=full
ENTRYPOINT ["/app/paxos_server"]
