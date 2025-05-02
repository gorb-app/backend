FROM debian:12-slim

RUN apt update && apt install libssl3 && rm -rf /var/lib/apt/lists/* /var/cache/apt/* /tmp/*

COPY target/release/backend-${TARGETARCH} /usr/bin/gorb-backend

COPY entrypoint.sh /usr/bin/entrypoint.sh

RUN useradd --create-home --home-dir /gorb gorb

USER gorb

ENV DATABASE_USERNAME="gorb"
ENV DATABASE_PASSWORD="gorb"
ENV DATABASE="gorb"
ENV DATABASE_HOST="localhost"
ENV DATABASE_PORT="5432"

ENTRYPOINT ["/usr/bin/entrypoint.sh"]
