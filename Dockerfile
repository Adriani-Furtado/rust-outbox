FROM rust:slim-bullseye as builder
COPY ./src/main.rs ./src/main.rs
COPY ./Cargo* .
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=builder /target/release/outbox-publisher .
CMD ["./outbox-publisher"]
