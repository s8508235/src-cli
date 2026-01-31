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

    /// Words per minute (default: 300)
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

    // Split text into words
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len();

    // Calculate duration per word based on WPM
    let seconds_per_word = 60.0 / args.wpm as f64;
    let total_duration = seconds_per_word * word_count as f64;

    println!("Creating video: {}", args.output);
    println!("Words: {} | WPM: {} | Duration per word: {:.2}s | Total: {:.2}s", 
             word_count, args.wpm, seconds_per_word, total_duration);

    // Build filter with multiple drawtext filters, each enabled for specific time range
    let mut filters = Vec::new();
    
    for (i, word) in words.iter().enumerate() {
        let start_time = i as f64 * seconds_per_word;
        let end_time = (i + 1) as f64 * seconds_per_word;
        
        // Escape word for FFmpeg
        let escaped_word = word
            .replace('\\', "\\\\")
            .replace('\'', "'\\''")
            .replace(':', "\\:");

        let drawtext = format!(
            "drawtext=text='{}':fontcolor=white:fontsize=120:x=(w-text_w)/2:y=(h-text_h)/2:enable='between(t,{},{})'",
            escaped_word, start_time, end_time
        );
        
        filters.push(drawtext);
    }

    // Combine all filters
    let filter_chain = filters.join(",");

    println!("Rendering video...");
    
    let output = Command::new("ffmpeg")
        .args([
            "-f", "lavfi",
            "-i", &format!("color=c=black:s=1920x1080:d={}", total_duration),
            "-vf", &filter_chain,
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
