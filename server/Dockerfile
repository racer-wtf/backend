FROM rust:latest
COPY . .
RUN cargo build --release --bin server
ARG DATABASE_URL
ARG RPC_URL
CMD ["./target/release/server"]
