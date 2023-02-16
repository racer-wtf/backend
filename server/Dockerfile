FROM rust:latest
COPY .. .
RUN cargo build --release --bin server
CMD ["./target/release/server"]
