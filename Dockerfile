FROM rust:latest as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN rustup component add rustfmt && \
    cargo build --release && \
    strip target/release/telegram-bot

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/telegram-bot /usr/local/bin/telegram-bot

ENV DATABASE_URL=sqlite:telegram_bot.db
ENV RUST_LOG=info

CMD ["telegram-bot"]
