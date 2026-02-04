use anyhow::Result;
use clap::Parser;

mod config;
mod ffmpeg;
/// Convert text to video using FFmpeg
#[derive(Parser, Debug)]
#[command(author="s8508235", version, about, long_about = None)]
struct Args {
    /// Input text (if not provided, reads from stdin)
    #[arg(short, long)]
    text: Option<String>,

    /// Output video file path
    #[arg(short, long, default_value = "output.mp4")]
    output: String,

    /// Words per minute (default: 300)
    #[arg(short, long, default_value = "300")]
    wpm: u32,

    /// Text color (default: #ffffee)
    #[arg(long, default_value = "#ffffee")]
    text_color: String,

    /// Background color (default: black)
    #[arg(long, default_value = "black")]
    bg_color: String,

    /// Show focus lines around the word
    #[arg(long, default_value_t = true)]
    focus_lines: std::primitive::bool,

    /// Focus line color (default: #1a1911)
    #[arg(long, default_value = "#1a1911")]
    secondary_color: String,

    /// Rest duration in seconds between sentences for blinking (default: 0.1)
    #[arg(long, default_value = "0.1")]
    rest_duration: f64,

    // local bgm location for webm
    #[arg(long, default_value = None)]
    bgm_location: Option<String>,

    // local font location for output text
    #[arg(long, default_value = None)]
    font_location: Option<String>,

    // overwrite output file if the same name file exists
    #[arg(long)]
    overwrite_output_file: Option<std::primitive::bool>,
}

fn main() -> Result<()> {
    // Check if ffmpeg is available
    ffmpeg::check_ffmpeg()?;

    let mut args = Args::parse();
    // overwrite config if args not present
    config::merge_config_with_args(&mut args);

    ffmpeg::generate_video(args)?;

    Ok(())
}
