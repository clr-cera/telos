FROM rust:latest as builder

WORKDIR /app
RUN apt-get update -y && apt-get install libssl-dev -y

COPY Cargo.toml Cargo.lock ./

# INstall the actual application
COPY src ./src
RUN cargo build --release

FROM debian:latest as runner
WORKDIR /app
RUN apt-get update -y && apt-get install openssl ca-certificates -y
COPY --from=builder /app/target/release/telos .
COPY .env .
CMD ["/app/telos"]
