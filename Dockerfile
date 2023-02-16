FROM rust:latest AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM alpine
COPY --from=builder /usr/src/app/target/release/server ./
CMD [ "./server" ]

FROM alpine
COPY --from=builder /usr/src/app/target/release/server ./
CMD [ "./indexer" ]
