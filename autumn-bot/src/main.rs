use std::env;

use poise::serenity_prelude as serenity;
use tracing::{debug, error, info};
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use rustls::crypto::ring::default_provider;
use sqlx::postgres::PgPoolOptions;

use autumn_core::{Data, Error};
use autumn_database::{Database, MIGRATOR};
use autumn_database::impls::cases::ensure_case_schema_compat;

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
    let db = Database::new(db_pool);

    let auto_run_migrations = env_bool("AUTO_RUN_MIGRATIONS", true);
    if auto_run_migrations {
        MIGRATOR.run(db.pool()).await?;
        info!("Database migrations applied.");

        ensure_case_schema_compat(&db).await?;
        info!("Case schema compatibility checks applied.");
    } else {
        info!("Auto migrations disabled (set AUTO_RUN_MIGRATIONS=true to run at startup).");
    }

    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: autumn_commands::commands(),
            on_error: |error| Box::pin(on_error(error)),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(autumn_utils::COMMAND_PREFIX.to_string()),
                mention_as_prefix: false,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Autumn has awoken!");

                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    serenity::GuildId::new(guild_id),
                )
                .await?;

                Ok(Data { db })
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
        Ok(value) => matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
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
                .send(
                    poise::CreateReply::default()
                        .ephemeral(true)
                        .embed(embed),
                )
                .await;
        }
        poise::FrameworkError::UnknownCommand { .. } => {
            debug!("unknown command invocation");
        }
        other => {
            error!(?other, "framework error");
        }
    }
}
