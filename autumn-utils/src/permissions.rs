use poise::serenity_prelude as serenity;

/// Convert a permission bitset into a sorted display list.
///
/// If `ADMINISTRATOR` is present, only `ADMINISTRATOR` is returned because
/// it implicitly grants all permissions.
pub fn permission_names(perms: serenity::Permissions) -> Vec<String> {
    if perms.contains(serenity::Permissions::ADMINISTRATOR) {
        return vec!["ADMINISTRATOR".to_owned()];
    }

    let mut names: Vec<String> = perms
        .iter_names()
        .map(|(name, _flag)| name.to_owned())
        .collect();
    names.sort_unstable();
    names
}

/// Resolve the invoking author's effective guild permissions for a message command.
///
/// Returns `Ok(None)` when the message is not from a guild context.
pub async fn resolve_user_permissions(
    http: &serenity::Http,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
) -> anyhow::Result<serenity::Permissions> {
    let guild = guild_id.to_partial_guild(http).await?;
    if guild.owner_id == user_id {
        return Ok(serenity::Permissions::all());
    }

    let member = guild_id.member(http, user_id).await?;
    let roles = guild_id.roles(http).await?;

    let mut resolved = serenity::Permissions::empty();
    let everyone_role_id = serenity::RoleId::new(guild_id.get());

    for role in roles.values() {
        if role.id == everyone_role_id || member.roles.contains(&role.id) {
            resolved |= role.permissions;
        }
    }

    Ok(resolved)
}

pub async fn has_user_permission(
    http: &serenity::Http,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    required: serenity::Permissions,
) -> anyhow::Result<bool> {
    let perms = resolve_user_permissions(http, guild_id, user_id).await?;

    Ok(perms.contains(serenity::Permissions::ADMINISTRATOR) || perms.contains(required))
}
