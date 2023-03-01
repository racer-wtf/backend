FROM rust:latest
COPY . .
ARG DATABASE_URL
RUN cargo build --release --bin indexer
CMD ["./target/release/indexer"]
