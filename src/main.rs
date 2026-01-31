use anyhow::{Context, Result};
use clap::Parser;
use std::io::{self, Read};
use std::process::Command;

/// Convert text to video using FFmpeg
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input text (if not provided, reads from stdin)
    #[arg(short, long)]
    text: Option<String>,

    /// Output video file path
    #[arg(short, long, default_value = "output.mp4")]
    output: String,

    /// Video duration in seconds (overrides WPM calculation)
    #[arg(short, long)]
    duration: Option<u32>,

    /// Words per minute (default: 300). Used to auto-calculate duration
    #[arg(short, long, default_value = "300")]
    wpm: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Get input text from argument or stdin
    let text = match args.text {
        Some(t) => t,
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read from stdin")?;
            buffer.trim().to_string()
        }
    };

    if text.is_empty() {
        anyhow::bail!("No text provided. Use --text or pipe text to stdin.");
    }

    // Calculate duration based on word count and WPM, or use explicit duration
    let word_count = text.split_whitespace().count() as u32;
    let duration = match args.duration {
        Some(d) => d,
        None => {
            // Calculate: (word_count / WPM) * 60 seconds, minimum 1 second
            let calculated = ((word_count as f64 / args.wpm as f64) * 60.0).ceil() as u32;
            calculated.max(1)
        }
    };

    // Escape text for FFmpeg filter
    let escaped_text = text
        .replace('\\', "\\\\")
        .replace('\'', "'\\''")
        .replace(':', "\\:");

    println!("Creating video: {}", args.output);
    println!("Text: \"{}\"", text);
    println!("Duration: {}s ({}wpm, {} words)", duration, args.wpm, word_count);
    
    let output = Command::new("ffmpeg")
        .args([
            "-f", "lavfi",
            "-i", &format!("color=c=black:s=1920x1080:d={}", duration),
            "-vf", &format!("drawtext=text='{}':fontcolor=white:fontsize=120:x=(w-text_w)/2:y=(h-text_h)/2", escaped_text),
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-y",
            &args.output,
        ])
        .output()
        .context("Failed to execute ffmpeg. Make sure ffmpeg is installed and in PATH.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg failed:\n{}", stderr);
    }

    println!("✓ Video created: {}", args.output);
    Ok(())
}
