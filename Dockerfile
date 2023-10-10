FROM rust:latest as builder

RUN rustup target add x86_64-unknown-linux-musl &&  apt-get update && apt-get install -y  musl-tools musl-dev &&  update-ca-certificates

WORKDIR /app

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/presence-bot ./
RUN chmod +x presence-bot
CMD ["/app/presence-bot"]