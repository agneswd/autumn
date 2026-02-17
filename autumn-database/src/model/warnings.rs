#[derive(Clone, Debug)]
pub struct WarningEntry {
    pub warned_at: u64,
    pub moderator_id: u64,
    pub reason: String,
}

#[derive(Clone, Copy, Debug)]
pub struct WarningRecord {
    pub warn_number: usize,
}
