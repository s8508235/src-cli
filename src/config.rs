use std::path::PathBuf;

use anyhow::{Context, Result};
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

    let content = std::fs::read_to_string(&config_path).context("Failed to read config file")?;

    let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

    Ok(config)
}

pub fn merge_config_with_args(args: &mut crate::Args) {
    // Load config and merge with CLI args (CLI args take precedence)
    let config = load_config().unwrap();
    // Only override if arg is at default value and config has a value

    // WPM: default is 300
    if args.wpm == 300
        && let Some(wpm) = config.wpm
    {
        args.wpm = wpm;
    }

    // Text color: default is "white"
    if args.text_color == "white"
        && let Some(color) = config.text_color
    {
        args.text_color = color.clone();
    }

    // Background color: default is "black"
    if args.bg_color == "black"
        && let Some(color) = config.bg_color
    {
        args.bg_color = color.clone();
    }

    // Secondary color: check if it's at default
    if args.secondary_color == "#1a1911"
        && let Some(color) = config.secondary_color
    {
        args.secondary_color = color;
    }

    // Rest duration: default is 0.5
    if (args.rest_duration - 0.5).abs() < f64::EPSILON
        && let Some(duration) = config.rest_duration
    {
        args.rest_duration = duration;
    }

    // Focus lines: default is true
    if args.focus_lines
        && let Some(focus) = config.focus_lines
    {
        args.focus_lines = focus;
    }

    // BGM location: only use config if not provided via CLI
    if args.bgm_location.is_none() {
        args.bgm_location = config.bgm_location.clone();
    }

    if args.font_location.is_none() {
        args.font_location = config.font_location.clone();
    }

    if args.overwrite_output_file.is_none() {
        args.overwrite_output_file = config.overwrite_output_file;
    }
}
