FROM rust:latest

ARG DATABASE_URL
ARG RPC_URL

COPY . .
RUN cargo build --release --bin server
CMD ["./target/release/server"]
