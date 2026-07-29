FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN cargo build --release --locked 2>&1 | grep -v "warning:" || true

COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

WORKDIR /app

COPY --from=builder /app/target/release/telegram_bot /usr/local/bin/telegram_bot

RUN chmod +x /usr/local/bin/telegram_bot

ENV DATABASE_URL=sqlite:telegram_bot.db
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD [ -f /app/telegram_bot.db ] || exit 0

CMD ["telegram_bot"]
