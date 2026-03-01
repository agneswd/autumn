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
- **Cache**: [deadpool-redis](https://github.com/bikeshedder/deadpool) (Redis)
- **AI/LLM**: [ollama-rs](https://github.com/pepperquack/ollama-rs) & [Ollama](https://ollama.com/)

### Website
- **Framework**: [React](https://react.dev/) & [Vite](https://vitejs.dev/)
- **Styling**: [Tailwind CSS v4](https://tailwindcss.com/)
- **Animations**: [GSAP](https://gsap.com/)
- **Icons**: [Lucide React](https://lucide.dev/)

## Setup

Autumn is primarily designed for private use, but you're welcome to self-host it.

A `Dockerfile` and `docker-compose.yml` are provided for containerized setup (recommended). Alternatively, Rust, PostgreSQL, and Redis can be installed manually.

Copy `.env.example` to `.env` and fill in the required variables:
- `DISCORD_TOKEN`
- `POSTGRES_PASSWORD`, `POSTGRES_MIGRATOR_PASSWORD`, `POSTGRES_APP_PASSWORD`
- `DATABASE_URL` — only needed for manual (non-container) setup (e.g. `postgres://username:password@localhost/autumn`)

### Containerized (Docker / Podman)

Works with Docker Compose v2+ and rootless Podman (`podman-compose`). Replace `docker compose` with `podman-compose` if using Podman.

```bash
touch SYSTEM_PROMPT.md          # custom LLM prompt — leave empty for the default
docker compose build
docker compose up -d postgres redis
docker compose run --rm migrator
docker compose up -d autumn-bot
```

To update after a `git pull`:
```bash
./docker/update.sh        # Docker
./docker/update-podman.sh # Podman
```

**LLM support (Ollama)** is opt-in via the `llm` profile. Set `OLLAMA_MODEL` in `.env` to the model you want, then:
```bash
docker compose --profile llm up -d ollama
docker compose exec ollama ollama pull llama3   # pull once; cached in volume
docker compose --profile llm up -d autumn-bot
```

### Manual

Rust must be installed along with PostgreSQL and Redis. Install `sqlx-cli` if you haven't already:
```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
```

Migrate the database:
```bash
cd autumn-database && sqlx migrate run
```

Run the bot:
```bash
cargo run --release
```

---

*Note: This project originally started using the `twilight` ecosystem for Discord API interactions before being refactored to use `serenity` and `poise`. You can find the original archived repository here: [rusty-twilight](https://github.com/agneswd/rusty-twilight).*