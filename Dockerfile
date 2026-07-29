FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/telegram_bot /usr/local/bin/telegram_bot

ENV DATABASE_URL=sqlite:telegram_bot.db
ENV RUST_LOG=info

CMD ["telegram_bot"]
