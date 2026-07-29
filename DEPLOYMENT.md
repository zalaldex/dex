# Deployment Guide

## Quick Start

1. Fork/clone this repository
2. Obtain your BOT_TOKEN from Telegram BotFather
3. Deploy to your chosen platform
4. Set `BOT_TOKEN` environment variable
5. Start using your bot!

## One-Click Deployment Links

### Railway
[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new?template=https%3A%2F%2Fgithub.com%2Fyourusername%2Ftelegram-bot-rs)

Steps:
1. Click deploy button
2. Add `BOT_TOKEN` environment variable
3. Click Deploy

### Render
1. Go to https://render.com
2. Click "New +" > "Web Service"
3. Connect repository
4. Set `BOT_TOKEN` in environment
5. Click "Create Web Service"

### Fly.io
```bash
fly launch
fly secrets set BOT_TOKEN="your_token"
fly deploy
```

### Koyeb
1. Visit https://koyeb.com
2. Connect GitHub
3. Select repository
4. Add `BOT_TOKEN` environment variable
5. Deploy

### Zeabur
1. Visit https://zeabur.com
2. Create new project
3. Import from GitHub
4. Add `BOT_TOKEN`
5. Deploy

### Northflank
1. Visit https://northflank.com
2. Create new project
3. Select Docker
4. Connect repository
5. Add `BOT_TOKEN`
6. Deploy

## Environment Variables

**Required:**
- `BOT_TOKEN` - Your Telegram bot token

**Optional:**
- `DATABASE_URL` - SQLite connection string (default: `sqlite:telegram_bot.db`)
- `RUST_LOG` - Log level (default: `info`)

## Verification

After deployment:

1. Open Telegram
2. Search for your bot
3. Send `/start`
4. Click Settings to verify statistics working
5. Send test messages in each mode

## Troubleshooting

### Bot not responding
- Check `BOT_TOKEN` is set correctly
- Check logs for errors
- Verify bot is running

### High memory usage
- Edit `src/main.rs` line with `max_connections(5)` → `max_connections(2)`
- Rebuild and redeploy

### Database errors
- Bot creates database automatically
- Ensure write permissions in deployment
- Check disk space

### Rate limited
- Telegram limits are 30 messages/second
- Bot handles this gracefully
- No user action needed

## Production Checklist

- [ ] `BOT_TOKEN` set in environment
- [ ] Database directory writable
- [ ] Logs visible in platform dashboard
- [ ] Bot responds to `/start`
- [ ] Settings page loads
- [ ] Each mode works
- [ ] Statistics track messages
- [ ] Handles large messages (auto-split)
- [ ] Gracefully recovers from errors

## Monitoring

### Logs
- **Railway**: Logs tab
- **Render**: Logs section
- **Fly.io**: `flyctl logs`
- **Koyeb**: Logs page
- **Zeabur**: Activity logs

### Health
- Bot automatically recovers from errors
- Long polling handles network interruptions
- SQLite WAL mode prevents database corruption

## Scaling

- Free tier: 1000s of users per day
- Database is SQLite (local)
- Each instance has independent database
- No shared state between instances

## Updates

Push to main branch and platform will auto-redeploy in seconds.

```bash
git add .
git commit -m "Your changes"
git push origin main
```

Platform auto-deployment enabled by default.

## Support

Check logs first, then review:
- README.md
- DEPLOYMENT.md
- GitHub Issues
