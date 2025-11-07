# syntax=docker/dockerfile:1.6

########################################
# Builder image
########################################
FROM rust:1.91.0-slim AS builder

WORKDIR /app

# Install build dependencies needed by some crates (ring, sqlx, etc.)
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation layers
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

# Copy the full project and build the release binary
COPY src ./src
COPY templates ./templates
COPY web ./web
COPY migrations ./migrations
COPY README.md CLAUDE.md Makefile ./
RUN cargo build --release --locked

########################################
# Runtime image
########################################
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV APP_USER=vpass
RUN useradd --system --create-home --user-group ${APP_USER}

WORKDIR /app

# Copy runtime artifacts
COPY --from=builder /app/target/release/vpass /usr/local/bin/vpass
COPY --from=builder /app/web ./web

# Useful defaults; override as needed
ENV PORT=3000 \
    RUST_LOG=info

EXPOSE 3000
USER ${APP_USER}

CMD ["vpass"]
