FROM alpine:latest

ARG BINARY_FILE=./target/x86_64-unknown-linux-musl/release/simple_webhook_schedule
COPY ${BINARY_FILE} ./simple_webhook_schedule

COPY ./schedules/ ./schedules/

ENV RUST_LOG=info

ENTRYPOINT ["./simple_webhook_schedule"]
