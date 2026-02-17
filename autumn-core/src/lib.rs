use autumn_database::Database;

pub type Error = anyhow::Error;

#[derive(Clone, Debug)]
pub struct Data {
    pub db: Database,
}

pub type Context<'a> = poise::Context<'a, Data, Error>;
