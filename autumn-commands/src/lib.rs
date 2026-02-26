pub mod moderation;
pub mod utility;

use autumn_core::{Data, Error};

pub struct CommandMeta {
    pub name: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
    pub usage: &'static str,
}

pub const COMMANDS: &[CommandMeta] = &[
    utility::ping::META,
    utility::universe::META,
    utility::help::META,
    utility::usage::META,
    utility::pagetest::META,
    moderation::aitoggle::META,
    moderation::ban::META,
    moderation::unban::META,
    moderation::kick::META,
    moderation::timeout::META,
    moderation::untimeout::META,
    moderation::warn::META,
    moderation::warnings::META,
    moderation::unwarn::META,
    moderation::purge::META,
    moderation::permissions::META,
    moderation::terminate::META,
    moderation::modlogs::META,
    moderation::modlogchannel::META,
    moderation::userlogs::META,
    moderation::userlogchannel::META,
    moderation::case::META,
    moderation::notes::META,
    moderation::wordfilter::META,
    moderation::escalation::META,
];

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        utility::ping::ping(),
        utility::universe::universe(),
        utility::help::help(),
        utility::usage::usage(),
        utility::pagetest::pagetest(),
        moderation::aitoggle::aitoggle(),
        moderation::ban::ban(),
        moderation::unban::unban(),
        moderation::kick::kick(),
        moderation::timeout::timeout(),
        moderation::untimeout::untimeout(),
        moderation::warn::warn(),
        moderation::warnings::warnings(),
        moderation::unwarn::unwarn(),
        moderation::purge::purge(),
        moderation::permissions::permissions(),
        moderation::terminate::terminate(),
        moderation::modlogs::modlogs(),
        moderation::modlogchannel::modlogchannel(),
        moderation::userlogs::userlogs(),
        moderation::userlogchannel::userlogchannel(),
        moderation::case::case(),
        moderation::notes::notes(),
        moderation::wordfilter::wordfilter(),
        moderation::escalation::escalation(),
    ]
}
