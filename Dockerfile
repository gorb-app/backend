FROM rust:bookworm AS builder

WORKDIR /src

COPY . .

RUN cargo build --release

FROM debian:12-slim

RUN apt update && apt install libssl3 && rm -rf /var/lib/apt/lists/* /var/cache/apt/* /tmp/*

COPY --from=builder /src/target/release/backend /usr/bin/gorb-backend

COPY entrypoint.sh /usr/bin/entrypoint.sh

RUN useradd --create-home --home-dir /gorb gorb

USER gorb

ENV DATABASE_USERNAME=gorb \
DATABASE_PASSWORD=gorb \
DATABASE=gorb \
DATABASE_HOST=database \
DATABASE_PORT=5432 \
CACHE_DB_HOST=valkey \
CACHE_DB_PORT=6379 \
BUNNY_API_KEY=your_storage_zone_password_here \
BUNNY_ENDPOINT=Frankfurt \
BUNNY_ZONE=gorb \
BUNNY_CDN_URL=https://cdn.gorb.app

ENTRYPOINT ["/usr/bin/entrypoint.sh"]
