#[path = "case/mod.rs"]
mod case_group;
#[path = "config/mod.rs"]
mod config_group;
#[path = "core/mod.rs"]
mod core_group;
#[path = "reversals/mod.rs"]
mod reversals_group;

pub use case_group::{case, modlogs, notes, warnings};
pub use config_group::{aitoggle, modlogchannel, permissions};
pub use core_group::{ban, kick, purge, terminate, timeout, warn};
pub use reversals_group::{unban, untimeout, unwarn};

mod embeds;
mod logging;
