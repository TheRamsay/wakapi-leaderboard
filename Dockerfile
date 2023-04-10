ARG project_name=wakapi-leaderboard

FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo clean
RUN cargo build --release

FROM debian:buster-slim as final
ARG project_name

RUN apt-get update
RUN apt-get install -y openssl ca-certificates

COPY --from=builder /app/target/release/$project_name ./app
USER 1000
CMD ["./app"]