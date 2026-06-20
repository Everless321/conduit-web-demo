# ---- build stage ----
# git: needed to fetch the conduit crates pinned via git in Cargo.toml.
# cc/libsqlite3-sys is vendored, so a C toolchain (in the rust image) is enough.
FROM rust:1.88-bookworm AS builder
WORKDIR /app

# Copy manifests + vendored deps + sources, then build OFFLINE (no network:
# all crates are vendored in ./vendor, redirected via .cargo/config.toml).
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY vendor ./vendor
COPY src ./src
RUN cargo build --release --locked --offline --bin conduit-web-demo

# ---- runtime stage ----
# russh is pure-Rust and SQLite is vendored, so we only need glibc + CA certs.
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/conduit-web-demo /usr/local/bin/conduit-web-demo

# Persist DB + auto-generated master key here (mount a volume on /data).
VOLUME ["/data"]
EXPOSE 8088

# Bind to 0.0.0.0 so the published port is reachable from the host.
ENV CONDUIT_WEB_BIND=0.0.0.0:8088 \
    CONDUIT_DB=/data/conduit-demo.db

ENTRYPOINT ["conduit-web-demo"]
