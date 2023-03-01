FROM rust:latest
COPY . .
ARG DATABASE_URL
RUN cargo build --release --bin server
CMD ["./target/release/server"]
