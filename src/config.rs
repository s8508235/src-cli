use std::path::PathBuf;

use anyhow::{Context, Ok, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub wpm: Option<u32>,
    pub text_color: Option<String>,
    pub bg_color: Option<String>,
    pub focus_color: Option<String>,
    pub secondary_color: Option<String>,
    pub rest_duration: Option<f64>,
    pub focus_lines: Option<bool>,
    pub bgm_location: Option<String>,
    pub font_location: Option<String>,
    pub overwrite_output_file: Option<bool>,
}

fn get_config_path() -> Result<PathBuf> {
    let home = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE")
            .or_else(|_| {
                std::env::var("HOMEDRIVE").and_then(|drive| {
                    std::env::var("HOMEPATH").map(|path| format!("{}{}", drive, path))
                })
            })
            .context("Could not find home directory")?
    } else {
        std::env::var("HOME").context("Could not find home directory")?
    };

    Ok(PathBuf::from(home).join(".src-cli.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to load config from {}", config_path.display()))?;

    Ok(config)
}

pub fn merge_config_with_args(args: &mut crate::Args) -> Result<()> {
    // Load config and merge with CLI args (CLI args take precedence)
    let mut config = load_config().context("Failed to load user configuration")?;
    // Only override if arg is at default value and config has a value

    // Scalar fields - use a helper function
    fn merge_scalar<T: PartialEq>(target: &mut T, default: T, source: Option<T>) {
        if *target == default
            && let Some(value) = source
        {
            *target = value;
        }
    }

    merge_scalar(&mut args.wpm, 300, config.wpm);
    merge_scalar(&mut args.text_color, "white".to_string(), config.text_color);
    merge_scalar(&mut args.bg_color, "black".to_string(), config.bg_color);
    merge_scalar(
        &mut args.secondary_color,
        "#1a1911".to_string(),
        config.secondary_color,
    );

    // Float with epsilon comparison
    const DEFAULT_REST_DURATION: f64 = 0.5;
    if (args.rest_duration - DEFAULT_REST_DURATION).abs() < f64::EPSILON
        && let Some(d) = config.rest_duration.take()
    {
        args.rest_duration = d;
    }

    // Boolean
    if args.focus_lines
        && let Some(f) = config.focus_lines.take()
    {
        args.focus_lines = f;
    }

    // Option fields - use get_or_insert
    args.bgm_location = args.bgm_location.take().or(config.bgm_location);
    args.font_location = args.font_location.take().or(config.font_location);
    args.overwrite_output_file = args.overwrite_output_file.or(config.overwrite_output_file);

    Ok(())
}
