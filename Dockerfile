FROM --platform=linux/amd64 debian:12-slim AS prep

WORKDIR /src

COPY target/release/backend backend-amd64
COPY target/aarch64-unknown-linux-gnu/release/backend backend-arm64

FROM debian:12-slim

ARG TARGETARCH

RUN apt update -y && apt install libssl3 ca-certificates -y && rm -rf /var/lib/apt/lists/* /var/cache/apt/* /tmp/*

COPY --from=prep /src/backend-${TARGETARCH} /usr/bin/gorb-backend

COPY entrypoint.sh /usr/bin/entrypoint.sh

RUN useradd --create-home --home-dir /gorb gorb

USER gorb

ENV WEB_FRONTEND_URL=https://gorb.app/web/ \
WEB_BASE_PATH=/api \
DATABASE_USERNAME=gorb \
DATABASE_PASSWORD=gorb \
DATABASE=gorb \
DATABASE_HOST=database \
DATABASE_PORT=5432 \
CACHE_DB_HOST=valkey \
CACHE_DB_PORT=6379 \
BUNNY_API_KEY=your_storage_zone_password_here \
BUNNY_ENDPOINT=Frankfurt \
BUNNY_ZONE=gorb \
BUNNY_CDN_URL=https://cdn.gorb.app \
MAIL_ADDRESS=noreply@gorb.app \
MAIL_TLS=tls \
SMTP_SERVER=mail.gorb.app \
SMTP_USERNAME=your_smtp_username \
SMTP_PASSWORD=your_smtp_password

ENTRYPOINT ["/usr/bin/entrypoint.sh"]
