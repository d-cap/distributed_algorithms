FROM rust:bookworm as builder

WORKDIR /app/log

COPY . /app/log

#RUN cargo test
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR app

COPY --from=builder /app/log/target/release/log_server /app

EXPOSE 8080

ENTRYPOINT ["/app/log_server"]
