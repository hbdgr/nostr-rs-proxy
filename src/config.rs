use serde::{Deserialize, Serialize};
use tracing::warn;

use config::{Config, ConfigError, File};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(unused)]
pub struct Network {
    pub port: u16,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(unused)]
pub struct Sources {
    pub relays: Vec<String>, // external relays addresses
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub network: Network,
    pub sources: Sources,
}

impl Settings {
    #[must_use]
    pub fn new(config_file_name: &Option<String>) -> Self {
        let default_settings = Self::default();
        // attempt to construct settings with file
        let from_file = Self::new_from_default(&default_settings, config_file_name);
        match from_file {
            Ok(f) => f,
            Err(e) => {
                warn!("Error reading config file ({:?})", e);
                default_settings
            }
        }
    }

    fn new_from_default(default: &Settings, config_file_name: &Option<String>) -> Result<Self, ConfigError> {
        let default_config_file_name = "config.toml".to_string();
        let config: &String = match config_file_name {
            Some(value) => value,
            None => &default_config_file_name
        };
        let builder = Config::builder();
        let config: Config = builder
        // use defaults
            .add_source(Config::try_from(default)?)
        // override with file contents
            .add_source(File::with_name(config))
            .build()?;
        let settings: Settings = config.try_deserialize()?;

        Ok(settings)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            network: Network {
                port: 8085,
                address: "0.0.0.0".to_owned(),
            },
            sources: Sources {
                relays: Vec::new(),
            },
        }
    }
}
