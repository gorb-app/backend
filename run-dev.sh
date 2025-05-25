#!/bin/sh

podman-compose --file compose.dev.yml up --build

echo "SHUTTING DOWN CONTAINERS"
podman container stop backend_backend_1 backend_database_1 backend_valkey_1

echo "DELETING CONTAINERS"
podman container rm backend_backend_1 backend_database_1 backend_valkey_1
