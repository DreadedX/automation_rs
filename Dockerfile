FROM rust:1.89 AS base
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo install cargo-chef --locked --version 0.1.71 && \
    cargo install cargo-auditable --locked --version 0.6.6
WORKDIR /app
COPY ./rust-toolchain.toml .
RUN rustup toolchain install

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
# HACK: Now we can use unstable feature while on stable rust!
ENV RUSTC_BOOTSTRAP=1
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ARG RELEASE_VERSION
ENV RELEASE_VERSION=${RELEASE_VERSION}
RUN cargo auditable build --release

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /app/target/release/automation /app/automation
ENV AUTOMATION__ENTRYPOINT=/app/config/config.lua
ENV LUA_PATH="/app/?.lua;;"
COPY ./config /app/config
CMD [ "/app/automation" ]
