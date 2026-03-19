FROM rust:alpine

WORKDIR /app
COPY . .
RUN cargo build --release && cp target/release/xargs . && rm -rf target
