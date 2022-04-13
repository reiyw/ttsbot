FROM lukemathwalker/cargo-chef:latest-rust-1.59-bullseye AS chef
WORKDIR /app

FROM chef AS planner
COPY src src
COPY Cargo.toml .
COPY Cargo.lock .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json

RUN apt-get update && apt-get install -y --no-install-recommends \
    libopus-dev=1.3.1-0.1 \
    ffmpeg=7:4.3.3-0+deb11u1 \
    ;
RUN cargo chef cook --release --recipe-path recipe.json

COPY src src
COPY Cargo.toml .
ENV DATABASE_URL dummy
RUN cargo build --release

FROM debian:bullseye-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    libopus-dev=1.3.1-0.1 \
    ffmpeg=7:4.3.3-0+deb11u1 \
    ca-certificates=20210119 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

COPY --from=builder /app/target/release/ttsbot /usr/local/bin

CMD ["/usr/local/bin/ttsbot"]
