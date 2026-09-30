# Production image for the backend. Build with the backend folder as context:
#   docker build -t coaching-backend backend
# Queries are checked against .sqlx at build time, so no database is needed here.

FROM rust:1.95-slim-trixie AS build
WORKDIR /app
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx
COPY migrations migrations
COPY src src
RUN cargo build --release --locked --bin coaching-backend

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/coaching-backend /usr/local/bin/coaching-backend
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=coaching_backend=info,tower_http=info
EXPOSE 8080
USER nobody
# Migrations are embedded in the binary and run on startup.
CMD ["coaching-backend"]
