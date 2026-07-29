mod modes;

use anyhow::Result;
use chrono::Utc;
use log::{error, info};
use modes::{Mode, ModeManager};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{KeyboardButton, ReplyKeyboardMarkup, Update, UserId},
    utils::command::BotCommands,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UserPreferences {
    user_id: i64,
    mode: String,
}

struct AppState {
    bot: Bot,
    db: SqlitePool,
    mode_manager: ModeManager,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "This bot converts text into different formats")]
enum Command {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "show settings")]
    Settings,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .init();

    dotenv::dotenv().ok();

    let bot_token = std::env::var("BOT_TOKEN")
        .expect("BOT_TOKEN environment variable not found");

    info!("Initializing database...");
    let db = init_db().await?;
    info!("Database initialized successfully");

    let bot = Bot::new(bot_token);
    let mode_manager = ModeManager::new();

    let state = Arc::new(AppState {
        bot: bot.clone(),
        db,
        mode_manager,
    });

    info!("Starting bot with long polling...");
    
    let mut dispatcher = Dispatcher::builder(bot, update_handler)
        .dependencies(dptree::deps![state])
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error occurred in the dispatcher:",
        ))
        .build();

    dispatcher.dispatch().await;

    Ok(())
}

async fn init_db() -> Result<SqlitePool> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:telegram_bot.db".to_string());

    let connect_options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_preferences (
            user_id INTEGER PRIMARY KEY,
            mode TEXT NOT NULL DEFAULT 'Word',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            message_count INTEGER NOT NULL DEFAULT 1,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES user_preferences(user_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stat_date DATE UNIQUE NOT NULL,
            total_messages INTEGER NOT NULL DEFAULT 0,
            unique_users INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[dptree::handler]
async fn update_handler(
    update: Update,
    bot: Bot,
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match update.kind {
        teloxide::types::UpdateKind::Message(msg) => {
            if let Some(user) = msg.from {
                let user_id = user.id;

                ensure_user_exists(&state.db, user_id.0 as i64).await?;
                track_message(&state.db, user_id.0 as i64).await?;

                if let Some(text) = msg.text() {
                    handle_command_or_message(&bot, &state, user_id, text.to_string()).await?;
                } else if let Some(caption) = msg.caption() {
                    handle_text_content(&bot, &state, user_id, caption.to_string()).await?;
                } else {
                    track_non_text_message(&state.db, user_id.0 as i64).await?;
                }
            }
        }
        teloxide::types::UpdateKind::CallbackQuery(query) => {
            if let Some(from) = query.from.clone() {
                handle_callback_query(&bot, &state, from.id, query).await?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_command_or_message(
    bot: &Bot,
    state: &Arc<AppState>,
    user_id: UserId,
    text: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if text == "/start" {
        send_start_message(bot, user_id, state).await?;
    } else if text == "/settings" {
        send_settings_message(bot, user_id, state).await?;
    } else {
        handle_text_content(bot, state, user_id, text).await?;
    }
    Ok(())
}

async fn handle_text_content(
    bot: &Bot,
    state: &Arc<AppState>,
    user_id: UserId,
    text: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = get_user_mode(&state.db, user_id.0 as i64).await?;
    let formatted = state.mode_manager.format(&text, &mode);

    send_formatted_message(bot, user_id, &formatted).await?;
    Ok(())
}

async fn send_start_message(
    bot: &Bot,
    user_id: UserId,
    state: &Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let markup = get_main_keyboard();
    bot.send_message(user_id, "Welcome to Text Formatter Bot! 🤖\n\nSend any text and I'll format it according to your selected mode.\n\nUse Settings to change the formatting mode.")
        .reply_markup(markup)
        .await?;
    Ok(())
}

async fn send_settings_message(
    bot: &Bot,
    user_id: UserId,
    state: &Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let current_mode = get_user_mode(&state.db, user_id.0 as i64).await?;
    let stats = get_user_stats(&state.db, user_id.0 as i64).await?;

    let message = format!(
        "⚙️ *Settings*\n\n*Current Mode:* `{}`\n\n📊 *Statistics*\n\n{}",
        current_mode, stats
    );

    let markup = mode_selection_keyboard(&current_mode);
    bot.send_message(user_id, message)
        .parse_mode(teloxide::types::ParseMode::Markdown)
        .reply_markup(markup)
        .await?;

    Ok(())
}

async fn handle_callback_query(
    bot: &Bot,
    state: &Arc<AppState>,
    user_id: UserId,
    query: teloxide::types::CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let callback_data = query.data.unwrap_or_default();

    if callback_data.starts_with("mode_") {
        let new_mode = callback_data.replace("mode_", "");
        if Mode::from_str(&new_mode).is_ok() {
            save_user_mode(&state.db, user_id.0 as i64, &new_mode).await?;
            bot.answer_callback_query(query.id).await?;
            send_settings_message(bot, user_id, state).await?;
        }
    } else if callback_data == "refresh_stats" {
        bot.answer_callback_query(query.id).await?;
        send_settings_message(bot, user_id, state).await?;
    }

    Ok(())
}

async fn send_formatted_message(
    bot: &Bot,
    user_id: UserId,
    formatted: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_MESSAGE_LENGTH: usize = 4096;

    for chunk in formatted {
        if chunk.len() <= MAX_MESSAGE_LENGTH {
            bot.send_message(user_id, chunk.clone()).await?;
        } else {
            let mut current_message = String::new();
            for line in chunk.lines() {
                let line_with_newline = format!("{}\n", line);
                if current_message.len() + line_with_newline.len() <= MAX_MESSAGE_LENGTH {
                    current_message.push_str(&line_with_newline);
                } else {
                    if !current_message.is_empty() {
                        bot.send_message(user_id, current_message.trim().to_string())
                            .await?;
                    }
                    current_message = line_with_newline;
                }
            }
            if !current_message.is_empty() {
                bot.send_message(user_id, current_message.trim().to_string())
                    .await?;
            }
        }
    }

    Ok(())
}

fn get_main_keyboard() -> ReplyKeyboardMarkup {
    ReplyKeyboardMarkup::default()
        .append_row(vec![
            KeyboardButton::new("📝 Start"),
            KeyboardButton::new("⚙️ Settings"),
        ])
        .resize_keyboard(true)
        .one_time_keyboard(false)
}

fn mode_selection_keyboard(current_mode: &str) -> teloxide::types::InlineKeyboardMarkup {
    use teloxide::types::InlineKeyboardButton;

    let modes = ["Word", "Sentence", "Paragraph", "Full"];
    let mut buttons = Vec::new();

    for mode in modes.iter() {
        let label = if *mode == current_mode {
            format!("✓ {}", mode)
        } else {
            mode.to_string()
        };
        buttons.push(vec![InlineKeyboardButton::callback(
            label,
            format!("mode_{}", mode),
        )]);
    }

    buttons.push(vec![InlineKeyboardButton::callback(
        "🔄 Refresh Stats",
        "refresh_stats".to_string(),
    )]);

    teloxide::types::InlineKeyboardMarkup::new(buttons)
}

async fn ensure_user_exists(db: &SqlitePool, user_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO user_preferences (user_id, mode)
        VALUES (?, 'Word')
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

async fn get_user_mode(db: &SqlitePool, user_id: i64) -> Result<String> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT mode FROM user_preferences WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    Ok(result.unwrap_or_else(|| "Word".to_string()))
}

async fn save_user_mode(db: &SqlitePool, user_id: i64, mode: &str) -> Result<()> {
    sqlx::query(
        "UPDATE user_preferences SET mode = ?, updated_at = CURRENT_TIMESTAMP WHERE user_id = ?",
    )
    .bind(mode)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

async fn track_message(db: &SqlitePool, user_id: i64) -> Result<()> {
    let today = Utc::now().format("%Y-%m-%d").to_string();

    sqlx::query(
        r#"
        INSERT INTO daily_stats (stat_date, total_messages, unique_users)
        VALUES (?, 1, 1)
        ON CONFLICT(stat_date) DO UPDATE SET 
            total_messages = total_messages + 1,
            unique_users = (SELECT COUNT(DISTINCT user_id) FROM message_stats WHERE DATE(timestamp) = ?)
        "#,
    )
    .bind(&today)
    .bind(&today)
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT INTO message_stats (user_id, message_count) VALUES (?, 1)",
    )
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

async fn track_non_text_message(db: &SqlitePool, user_id: i64) -> Result<()> {
    track_message(db, user_id).await?;
    Ok(())
}

async fn get_user_stats(db: &SqlitePool, user_id: i64) -> Result<String> {
    let today_messages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM message_stats WHERE user_id = ? AND DATE(timestamp) = DATE('now')",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let last_24h_messages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM message_stats WHERE user_id = ? AND timestamp > datetime('now', '-1 day')",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let last_7d_messages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM message_stats WHERE user_id = ? AND timestamp > datetime('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let last_30d_messages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM message_stats WHERE user_id = ? AND timestamp > datetime('now', '-30 days')",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let last_year_messages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM message_stats WHERE user_id = ? AND timestamp > datetime('now', '-1 year')",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let lifetime_messages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM message_stats WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let total_unique_users: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM user_preferences",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let active_users: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM message_stats WHERE timestamp > datetime('now', '-7 days')",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    Ok(format!(
        "Global Stats:\n`Active Users (7d):` {}\n`Total Users:` {}\n\nYour Stats:\n`Today:` {}\n`24h:` {}\n`7d:` {}\n`30d:` {}\n`1y:` {}\n`Lifetime:` {}",
        active_users, total_unique_users, today_messages, last_24h_messages, last_7d_messages, last_30d_messages, last_year_messages, lifetime_messages
    ))
}
