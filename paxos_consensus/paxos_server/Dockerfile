FROM rust:bookworm as builder

WORKDIR /app/consensus

COPY . /app/consensus

#RUN cargo test
RUN cargo build --release

#FROM gcr.io/distroless/cc
FROM rust:bookworm
RUN apt-get update -y
RUN apt-get install -y iputils-ping

WORKDIR app

COPY --from=builder /app/consensus/target/release/paxos_server /app

EXPOSE 8080

ENTRYPOINT ["/app/paxos_server"]
