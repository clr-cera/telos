FROM rust:latest AS builder

WORKDIR /app
RUN apt-get update -y && apt-get install libssl-dev -y

COPY Cargo.toml Cargo.lock ./

# Install the actual application
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:latest AS runner
WORKDIR /app
RUN apt-get update -y && apt-get install openssl ca-certificates -y
COPY --from=builder /app/target/release/telos .
CMD ["/app/telos"]
