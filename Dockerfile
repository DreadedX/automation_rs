FROM rust:latest AS build

# Create user
ENV USER=automation
ENV UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

# Create basic project structure
RUN cargo new --bin /app
RUN cargo new --lib /app/impl_cast && truncate -s 0 /app/impl_cast/src/lib.rs
RUN cargo new --lib /app/google-home

# Get the correct version of the compiler
RUN rustup default nightly

# Copy cargo config
COPY .cargo/config.toml /app/.cargo/config.toml

# Copy the Cargo.toml files
COPY impl_cast/Cargo.toml /app/impl_cast
COPY google-home/Cargo.toml /app/google-home
COPY Cargo.toml Cargo.lock /app/

# Download and build all the dependencies
WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

# Build impl_cast
COPY impl_cast/src/ /app/impl_cast/src/
RUN --mount=type=cache,target=/usr/local/cargo/registry set -e; touch /app/impl_cast/src/lib.rs; cargo build --release --package impl_cast

# Build google-home
COPY google-home/src/ /app/google-home/src/
RUN --mount=type=cache,target=/usr/local/cargo/registry set -e; touch /app/google-home/src/lib.rs; cargo build --release --package google-home

# Build automation
COPY src/ /app/src/
RUN --mount=type=cache,target=/usr/local/cargo/registry set -e; touch /app/src/main.rs /app/src/lib.rs /app/google-home/src/lib.rs /app/impl_cast/src/lib.rs; cargo build --release

CMD ["/app/target/release/automation"]


# FINAL IMAGE
FROM gcr.io/distroless/cc

COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

ENV AUTOMATION_CONFIG=/app/config.toml
COPY config/config.toml /app/config.toml

WORKDIR /app
COPY --from=build /app/target/x86_64-unknown-linux-gnu/release/automation ./

USER automation:automation

CMD ["/app/automation"]
