FROM rust:buster as builder

WORKDIR /app/consensus

COPY . /app/consensus

RUN cargo test
RUN cargo build --release

FROM gcr.io/distroless/cc

WORKDIR app

COPY --from=builder /app/consensus/target/release/paxos_consensus /app

EXPOSE 8080

ENTRYPOINT ["/app/paxos_consensus"]
