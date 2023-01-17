FROM rustlang/rust:nightly-slim AS build

RUN cargo new --bin /app
RUN cargo new --lib /app/impl_cast
RUN cargo new --lib /app/google-home

COPY impl_cast/Cargo.toml /app/impl_cast
COPY google-home/Cargo.toml /app/google-home
COPY Cargo.toml Cargo.lock /app/

WORKDIR /app/
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

COPY impl_cast/src/ /app/impl_cast/src/
COPY google-home/src/ /app/google-home/src/
COPY src/ /app/src/

RUN --mount=type=cache,target=/usr/local/cargo/registry set -e; touch /app/src/main.rs /app/src/lib.rs /app/google-home/src/lib.rs /app/impl_cast/src/lib.rs; cargo build --release

CMD ["/app/target/release/automation"]
