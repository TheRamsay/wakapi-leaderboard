ARG project_name=wakapi-leaderboard

FROM rust:bookworm AS builder

WORKDIR /app

COPY . .

RUN cargo clean
RUN cargo build --release

FROM debian:bookworm-slim as final
ARG project_name

RUN apt-get update
RUN apt-get install -y openssl ca-certificates

COPY --from=builder /app/target/release/$project_name ./app
USER 1000
CMD ["./app"]