use autumn_database::Database;
use autumn_llm::LlmService;

pub type Error = anyhow::Error;

#[derive(Clone, Debug)]
pub struct Data {
    pub db: Database,
    pub llm: Option<LlmService>,
}

pub type Context<'a> = poise::Context<'a, Data, Error>;
