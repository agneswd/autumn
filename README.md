# Autumn

A general-purpose Discord moderation bot written in Rust, Serenity, and Poise.

Made for fun, private use, and open-source exploration.

## Features
- **Moderation**: Ban, kick, timeout, and warn users (`!ban`, `!kick`, `!timeout`, `!warn`)
- **Case Management**: Track and manage moderation cases and user notes (`!case`, `!notes`)
- **Message Purging**: Bulk delete messages with various filters (`!purge`)
- **Modlogs**: Configure and log moderation actions to a specific channel (`!modlogchannel`)
- **Utilities**: Helpful commands like `!ping`, `!help`, and `!usage`
- **Optional LLM Chat Integration**: AI-powered chat capabilities using Ollama

All commands are supported as prefix commands as well as slash commands.

With the `!help` command, the bot will provide a list of all available commands and explain how to use them.

## Internals

### Bot
- **Discord API**: [serenity](https://github.com/serenity-rs/serenity) & [poise](https://github.com/serenity-rs/poise)
- **Database**: [sqlx](https://github.com/launchbadge/sqlx) (PostgreSQL)
- **AI/LLM**: [ollama-rs](https://github.com/pepperquack/ollama-rs) & [Ollama](https://ollama.com/)

### Website
- **Framework**: [React](https://react.dev/) & [Vite](https://vitejs.dev/)
- **Styling**: [Tailwind CSS v4](https://tailwindcss.com/)
- **Animations**: [GSAP](https://gsap.com/)
- **Icons**: [Lucide React](https://lucide.dev/)

## Setup

While Autumn is primarily designed for private use, you are welcome to self-host it if you'd like to explore the codebase or run your own instance.

Rust must be installed, and additionally, PostgreSQL must be installed and running. If you plan to use the LLM chat features, you will also need to download and install [Ollama](https://ollama.com/).

Create a new file named `.env` in the root directory and provide all of its variables. The most important ones are:
- `DISCORD_TOKEN`
- `DATABASE_URL` (e.g., `postgres://username:password@localhost/autumn`)
- `DISCORD_GUILD_ID`
- `OLLAMA_HOST` (e.g., `http://127.0.0.1`)
- `OLLAMA_PORT` (e.g., `11434`)
- `OLLAMA_MODEL` (e.g., `llama3`)

Optional Redis cache variables:
- `REDIS_ENABLED` (`true`/`false`, default: `false`)
- `REDIS_URL` (e.g., `redis://127.0.0.1:6379`)
- `REDIS_KEY_PREFIX` (default: `autumn:prod`)

If Redis is enabled but unavailable or misconfigured, Autumn automatically falls back to database-only mode.

*(Optional)* The bot comes with a default system prompt for the LLM integration. If you want to use a custom prompt, simply create a `SYSTEM_PROMPT.md` file in the root directory and write your custom instructions there.

Next, install `sqlx-cli` if you haven't already. You can do so with:
```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
```

Then navigate to the `autumn-database` directory and migrate the database:
```bash
cd autumn-database
sqlx migrate run
```
*This command will complain if the `DATABASE_URL` variable in `.env` is not correct.*

And finally, you can compile and run the bot with:
```bash
cargo run
```
To make the bot run faster (but compiling takes longer), use:
```bash
cargo run --release
```

---

*Note: This project originally started using the `twilight` ecosystem for Discord API interactions before being refactored to use `serenity` and `poise`. You can find the original archived repository here: [rusty-twilight](https://github.com/agneswd/rusty-twilight).*