FROM lukemathwalker/cargo-chef:latest-rust-1.65.0 AS chef
RUN apt-get update --yes \
    && apt-get install --yes --no-install-recommends lld clang protobuf-compiler
WORKDIR /app

# compute a lock-like file for our project
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# build the project
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release

FROM debian:bullseye-slim AS runtime
RUN apt-get update --yes \
    && apt-get install --yes --no-install-recommends openssl ca-certificates \
    && apt-get autoremove --yes \
    && apt-get clean --yes \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/zero2prod zero2prod
ENTRYPOINT ["/app/zero2prod"]
