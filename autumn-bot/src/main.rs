use std::env;

use poise::serenity_prelude as serenity;
use tracing::{debug, error, info};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use rustls::crypto::ring::default_provider;
use sqlx::postgres::PgPoolOptions;

use autumn_core::{Data, Error};
use autumn_database::impls::ai_config::get_llm_enabled;
use autumn_database::impls::cases::ensure_case_schema_compat;
use autumn_database::impls::llm_chat::insert_llm_chat_message;
use autumn_database::{Database, MIGRATOR};
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
    let db = Database::new(db_pool);
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

                Ok(Data { db, llm })
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
    let serenity::FullEvent::Message { new_message } = event else {
        return Ok(());
    };

    if new_message.author.bot || new_message.webhook_id.is_some() {
        return Ok(());
    }

    let Some(guild_id) = new_message.guild_id else {
        return Ok(());
    };

    let Some(llm) = data.llm.as_ref() else {
        return Ok(());
    };

    let llm_enabled = match get_llm_enabled(&data.db, guild_id.get()).await {
        Ok(enabled) => enabled,
        Err(source) => {
            error!(?source, "failed to read guild AI config");
            return Ok(());
        }
    };

    if !llm_enabled {
        return Ok(());
    }

    let mentions_bot = match new_message.mentions_me(ctx).await {
        Ok(value) => value,
        Err(source) => {
            error!(?source, "failed to evaluate bot mention");
            false
        }
    };

    if !mentions_bot {
        return Ok(());
    }

    let bot_user_id = ctx.cache.current_user().id;
    let author_display_name = message_display_name(new_message);
    let bot_display_name = ctx.cache.current_user().name.clone();
    let prompt = strip_bot_mention(&new_message.content, bot_user_id)
        .trim()
        .to_owned();

    if prompt.is_empty() {
        new_message.reply(&ctx.http, "a?").await?;
        return Ok(());
    }

    let _ = new_message.channel_id.broadcast_typing(&ctx.http).await;

    let llm_reply = match llm
        .generate_channel_reply(
            &data.db,
            guild_id.get(),
            new_message.channel_id.get(),
            &prompt,
            &author_display_name,
        )
        .await
    {
        Ok(content) if !content.trim().is_empty() => content,
        Ok(_) => "I couldn't generate a useful response for that. Try rephrasing?".to_owned(),
        Err(source) => {
            error!(?source, "llm reply generation failed");
            new_message
                .reply(&ctx.http, "I ran into an LLM error. Try again in a moment.")
                .await?;
            return Ok(());
        }
    };

    if let Err(source) = insert_llm_chat_message(
        &data.db,
        guild_id.get(),
        new_message.channel_id.get(),
        new_message.author.id.get(),
        Some(author_display_name.as_str()),
        "user",
        &prompt,
    )
    .await
    {
        error!(?source, "failed to persist user llm chat message");
    }

    new_message.reply(&ctx.http, &llm_reply).await?;

    if let Err(source) = insert_llm_chat_message(
        &data.db,
        guild_id.get(),
        new_message.channel_id.get(),
        bot_user_id.get(),
        Some(bot_display_name.as_str()),
        "assistant",
        &llm_reply,
    )
    .await
    {
        error!(?source, "failed to persist assistant llm chat message");
    }

    Ok(())
}

fn strip_bot_mention(content: &str, bot_user_id: serenity::UserId) -> String {
    content
        .replace(&format!("<@{}>", bot_user_id.get()), "")
        .replace(&format!("<@!{}>", bot_user_id.get()), "")
        .trim()
        .to_owned()
}

fn message_display_name(message: &serenity::Message) -> String {
    if let Some(member) = &message.member
        && let Some(nick) = &member.nick
        && !nick.trim().is_empty()
    {
        return nick.clone();
    }

    if let Some(global_name) = &message.author.global_name
        && !global_name.trim().is_empty()
    {
        return global_name.clone();
    }

    message.author.name.clone()
}
