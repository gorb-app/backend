# backend

You can run and test the latest backend by using the docker image based on main

## docker compose file
```yml
version: '3.5'
volumes:
  gorb-backend:
  gorb-database:
networks:
  gorb:
services:
  backend:
    image: git.gorb.app/gorb/backend:main
    restart: always
    ports:
      - 8080:8080
    networks:
      - gorb
    volumes:
      - gorb-backend:/gorb
    environment:
      #- RUST_LOG=debug
      - DATABASE_USERNAME=gorb
      - DATABASE_PASSWORD=gorb
      - DATABASE=gorb
      - DATABASE_HOST=database
      - DATABASE_PORT=5432
  database:
    image: postgres:16
    restart: always
    networks:
      - gorb
    volumes:
      - gorb-database:/var/lib/postgresql/data
    environment:
      - POSTGRES_USER=gorb
      - POSTGRES_PASSWORD=gorb
      - POSTGRES_DB=gorb
```
