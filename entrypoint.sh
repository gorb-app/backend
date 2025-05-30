#!/bin/sh

if [ ! -d "/gorb/config" ]; then
    mkdir /gorb/config
fi

if [ ! -d "/gorb/logs" ]; then
    mkdir /gorb/logs
fi

if [ ! -f "/gorb/config/config.toml" ]; then
cat > /gorb/config/config.toml <<EOF
[web]
frontend_url = "${WEB_FRONTEND_URL}"
base_path = "${WEB_BASE_PATH}"

[database]
username = "${DATABASE_USERNAME}"
password = "${DATABASE_PASSWORD}"
database = "${DATABASE}"
host = "${DATABASE_HOST}"
port = ${DATABASE_PORT}

[cache_database]
host = "${CACHE_DB_HOST}"
port = ${CACHE_DB_PORT}

[bunny]
api_key = "${BUNNY_API_KEY}"
endpoint = "${BUNNY_ENDPOINT}"
storage_zone = "${BUNNY_ZONE}"
cdn_url = "${BUNNY_CDN_URL}"

[mail]
address = "${MAIL_ADDRESS}"
tls = "${MAIL_TLS}"

[mail.smtp]
server = "${SMTP_SERVER}"
username = "${SMTP_USERNAME}"
password = "${SMTP_PASSWORD}"

EOF
fi

rotate_log() {
  LOGFILE="$1"
  BASENAME=$(basename "$LOGFILE" .log)
  DIRNAME=$(dirname "$LOGFILE")

  if [ -f "$LOGFILE" ]; then
    # Find the next available number
    i=1
    while [ -f "$DIRNAME/${BASENAME}.${i}.log.gz" ]; do
      i=$((i + 1))
    done

    gzip "$LOGFILE"
    mv "${LOGFILE}.gz" "$DIRNAME/${BASENAME}.${i}.log.gz"
  fi
}

rotate_log "/gorb/logs/backend.log"

# Give the DB time to start up before connecting
sleep 5

/usr/bin/gorb-backend --config /gorb/config/config.toml 2>&1 | tee /gorb/logs/backend.log
