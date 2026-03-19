FROM rust:alpine

WORKDIR /app
COPY . .
RUN cargo build --release
