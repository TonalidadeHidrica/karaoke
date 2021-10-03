use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
pub struct Config {
    pub font_path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Config, ConfigLoadError> {
        let mut s = String::new();
        BufReader::new(File::open("config.toml")?).read_to_string(&mut s)?;
        Ok(toml::from_str(&s)?)
    }
}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),
    #[error("{0}")]
    IllegalConfigEntry(#[from] toml::de::Error),
}
