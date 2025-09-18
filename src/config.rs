use actix_settings::BasicSettings;
use serde::{Deserialize, Serialize};
use toml::Table;

pub type CompleteSettings = BasicSettings<Config>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub(crate) auth: String,
    pub(crate) logfile: String,
    pub(crate) commands: Table,
}
