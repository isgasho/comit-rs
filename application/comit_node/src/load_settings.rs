use crate::settings::ComitNodeSettings;
use config::ConfigError;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_path: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<ComitNodeSettings, ConfigError> {
    match opt.config_path {
        Some(config_path) => {
            let config_path = validate_path(config_path)?;
            let default_config = Path::join(&config_path, "default");
            ComitNodeSettings::read(default_config)
        }
        None => match directories::UserDirs::new() {
            None => Err(ConfigError::Message(
                "Unable to determine user's home directory".to_string(),
            )),
            Some(dirs) => {
                let config_path = Path::join(dirs.home_dir(), ".config/comit_node/default.toml");
                if config_path.exists() {
                    ComitNodeSettings::read(config_path)
                } else {
                    log::info!("config file was neither provided nor found at default location, generating default config at: {:?}", config_path);
                    ComitNodeSettings::default().write_to(config_path)
                }
            }
        },
    }
}

fn validate_path(path: PathBuf) -> Result<PathBuf, ConfigError> {
    match std::fs::metadata(path.clone()) {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(path)
            } else {
                Err(ConfigError::Message(format!(
                    "Config path is expected to be a directory: {:?}",
                    path
                )))
            }
        }
        Err(e) => Err(ConfigError::Message(format!(
            "Cannot access config path {:?}: {:?}",
            path, e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::load_settings::{load_settings, Opt};
    use spectral::prelude::*;

    #[test]
    fn can_find_config_path() {
        let opt = Opt {
            config_path: Some("./config".into()),
        };
        let result = load_settings(opt);
        assert_that(&result).is_ok();
    }

    #[test]
    fn cannot_find_config_path_should_return_error() {
        let opt = Opt {
            config_path: Some("./invalid_config_dir".into()),
        };
        let result = load_settings(opt);
        assert_that(&result).is_err();
    }

    #[test]
    fn no_config_provided_should_start_fine() {
        let opt = Opt { config_path: None };
        let result = load_settings(opt);
        assert_that(&result).is_ok();
    }
}
