FROM rust:1.95-alpine AS base
RUN cargo install cargo-chef --locked --version 0.1.71 && \
    cargo install cargo-auditable --locked --version 0.6.6
WORKDIR /app
RUN rustup toolchain install

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
RUN apk add --no-cache g++=15.2.0-r2 cmake=4.1.3-r0 make=4.4.1-r3
# HACK: Now we can use unstable feature while on stable rust!
ENV RUSTC_BOOTSTRAP=1
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ARG RELEASE_VERSION
ENV RELEASE_VERSION=${RELEASE_VERSION}
RUN cargo auditable build --release


FROM gcr.io/distroless/static-debian13:nonroot AS runtime
COPY --from=builder /app/target/release/automation /app/automation
ENV AUTOMATION__ENTRYPOINT=/app/config/config.lua
ENV LUA_PATH="/app/?.lua;;"
COPY ./config /app/config
CMD [ "/app/automation" ]
