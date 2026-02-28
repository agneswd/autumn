mod events;

use std::env;
use std::time::Duration;

use poise::serenity_prelude as serenity;
use tracing::{debug, error, info, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use rustls::crypto::ring::default_provider;
use sqlx::postgres::PgPoolOptions;

use autumn_core::{Data, Error};
use autumn_database::{
    CacheService, Database, MIGRATOR, cache::DEFAULT_LLM_MENTION_RATE_LIMIT_MAX_HITS,
    cache::DEFAULT_LLM_MENTION_RATE_LIMIT_WINDOW,
};
use autumn_llm::LlmService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(filter_fn(|metadata| {
        let target = metadata.target();

        let within_info_level = *metadata.level() <= tracing::Level::INFO;
        if !within_info_level {
            return false;
        }

        !(target.starts_with("serenity::gateway::bridge::shard_manager")
            || target.starts_with("serenity::gateway::bridge::shard_runner"))
    }));

    tracing_subscriber::registry().with(fmt_layer).init();

    default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls ring provider"))?;

    // Load the .env file
    dotenvy::dotenv().ok();

    // Store Discord Bot Token
    let token = env::var("DISCORD_TOKEN")?;
    let database_url = env::var("DATABASE_URL")?;
    let guild_id = env::var("DISCORD_GUILD_ID")?.parse::<u64>()?;

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    info!("PostgreSQL connection established.");

    let redis_enabled = env_bool("REDIS_ENABLED", false);
    let redis_key_prefix =
        env::var("REDIS_KEY_PREFIX").unwrap_or_else(|_| "autumn:prod".to_string());

    let mut cache = if redis_enabled {
        match env::var("REDIS_URL") {
            Ok(redis_url) => match CacheService::redis(&redis_url, redis_key_prefix.clone()) {
                Ok(cache) => {
                    info!(key_prefix = %redis_key_prefix, "Redis cache enabled.");
                    cache
                }
                Err(err) => {
                    warn!(?err, key_prefix = %redis_key_prefix, "Failed to initialize Redis cache; continuing with DB-only mode.");
                    CacheService::disabled(redis_key_prefix.clone())
                }
            },
            Err(_) => {
                warn!(key_prefix = %redis_key_prefix, "REDIS_ENABLED=true but REDIS_URL is missing; continuing with DB-only mode.");
                CacheService::disabled(redis_key_prefix.clone())
            }
        }
    } else {
        info!("Redis cache disabled (set REDIS_ENABLED=true to enable).");
        CacheService::disabled(redis_key_prefix.clone())
    };

    let llm_ratelimit_window_seconds = env_u64(
        "LLM_RATELIMIT_WINDOW_SECONDS",
        DEFAULT_LLM_MENTION_RATE_LIMIT_WINDOW.as_secs(),
    );
    let llm_ratelimit_max_hits = env_u64(
        "LLM_RATELIMIT_MAX_HITS",
        DEFAULT_LLM_MENTION_RATE_LIMIT_MAX_HITS,
    );
    cache.configure_llm_rate_limit(
        Duration::from_secs(llm_ratelimit_window_seconds),
        llm_ratelimit_max_hits,
    );
    info!(
        llm_ratelimit_window_seconds = cache.llm_rate_limit_window().as_secs(),
        llm_ratelimit_max_hits = cache.llm_rate_limit_max_hits(),
        "LLM rate limit configured."
    );

    if cache.is_redis_enabled() {
        if let Err(err) = cache.ping().await {
            warn!(
                ?err,
                "Redis cache ping failed; cache operations will continue with fallback behavior."
            );
        } else {
            info!("Redis cache health check passed.");
        }
    }

    let db = Database::with_cache(db_pool, cache);

    let llm = LlmService::from_env_optional()?;
    if llm.is_some() {
        info!("LLM integration enabled.");
    } else {
        info!("LLM integration disabled (missing/empty OLLAMA_* vars or OLLAMA_ENABLED=false).");
    }

    let auto_run_migrations = env_bool("AUTO_RUN_MIGRATIONS", true);
    if auto_run_migrations {
        MIGRATOR.run(db.pool()).await?;
        info!("Database migrations applied.");
    } else {
        info!("Auto migrations disabled (set AUTO_RUN_MIGRATIONS=true to run at startup).");
    }

    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: autumn_commands::commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(handle_event(ctx, event, framework, data))
            },
            on_error: |error| Box::pin(on_error(error)),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(autumn_utils::COMMAND_PREFIX.to_string()),
                mention_as_prefix: false,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let db = db.clone();
            let llm = llm.clone();
            Box::pin(async move {
                info!("Autumn has awoken!");

                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    serenity::GuildId::new(guild_id),
                )
                .await?;

                Ok(Data {
                    db,
                    llm,
                    suppressed_deletes: Default::default(),
                })
            })
        })
        .build();

    info!("Autumn is connecting...");

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    client.start().await?;
    Ok(())
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    match env::var(key) {
        Ok(value) => value.trim().parse::<u64>().unwrap_or(default),
        Err(_) => default,
    }
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!(?error, "command error");

            let embed = serenity::CreateEmbed::new()
                .title("Command Error")
                .description("Something went wrong while running this command.")
                .color(autumn_utils::embed::DEFAULT_EMBED_COLOR);

            let _ = ctx
                .send(poise::CreateReply::default().ephemeral(true).embed(embed))
                .await;
        }
        poise::FrameworkError::ArgumentParse { ctx, input, .. } => {
            let usage = format!("Usage: `!{}`", ctx.command().qualified_name);
            let description = if let Some(input) = input {
                format!("Invalid argument: `{}`\n{}", input, usage)
            } else {
                format!("Missing required argument.\n{}", usage)
            };

            let _ = ctx.say(description).await;
        }
        poise::FrameworkError::UnknownCommand { .. } => {
            debug!("unknown command invocation");
        }
        other => {
            error!(?other, "framework error");
        }
    }
}

async fn handle_event(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            events::word_filter::handle_message_word_filter(ctx, data, new_message).await;
            events::userlog::handle_message_create_userlog(data, new_message).await;
            events::llm_events::handle_message_mention_llm(ctx, data, new_message).await?;
        }
        serenity::FullEvent::MessageUpdate { event, .. } => {
            events::userlog::handle_message_update_userlog(ctx, data, event).await;
        }
        serenity::FullEvent::MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id: Some(guild_id),
            ..
        } => {
            events::userlog::handle_message_delete_userlog(
                ctx,
                data,
                *guild_id,
                *channel_id,
                *deleted_message_id,
            )
            .await;
        }
        serenity::FullEvent::MessageDeleteBulk {
            channel_id,
            multiple_deleted_messages_ids,
            guild_id: Some(guild_id),
            ..
        } => {
            for message_id in multiple_deleted_messages_ids {
                events::userlog::handle_message_delete_userlog(
                    ctx,
                    data,
                    *guild_id,
                    *channel_id,
                    *message_id,
                )
                .await;
            }
        }
        _ => {}
    }

    Ok(())
}
