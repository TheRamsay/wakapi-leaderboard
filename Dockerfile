ARG project_name=wakapi-leaderboard

FROM rust:latest AS builder
ARG project_name

RUN apt update

WORKDIR /usr/src
RUN USER=root cargo new ${project_name}
WORKDIR /usr/src/${project_name}
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN rm src/*.rs

COPY src ./src
RUN touch ./src/main.rs && cargo build --release

FROM debian:buster-slim

ARG project_name
RUN apt-get update \
    && apt-get install -y openssl ca-certificates
COPY --from=builder /usr/src/$project_name/target/release/$project_name ./app
USER 1000
CMD ["./app"]