use anyhow::{Context, Result};
use clap::Parser;
use std::io::{self, Read};
use std::process::Command;
use std::fs;

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

    /// Frames per second
    #[arg(long, default_value = "30")]
    fps: u32,
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
    let frames_per_word = (seconds_per_word * args.fps as f64).round() as u32;

    println!("Creating video: {}", args.output);
    println!("Words: {} | WPM: {} | Duration per word: {:.2}s | Frames per word: {}", 
             word_count, args.wpm, seconds_per_word, frames_per_word);

    // Create images for each word
    let temp_dir = std::env::temp_dir().join("text-to-video");
    fs::create_dir_all(&temp_dir)?;

    let mut frame_number = 0;
    
    for word in &words {
        // Escape word for FFmpeg
        let escaped_word = word
            .replace('\\', "\\\\")
            .replace('\'', "'\\''")
            .replace(':', "\\:");

        // Generate frames for this word
        for _ in 0..frames_per_word {
            let frame_path = temp_dir.join(format!("frame_{:06}.png", frame_number));
            
            let output = Command::new("ffmpeg")
                .args([
                    "-f", "lavfi",
                    "-i", "color=c=black:s=1920x1080:d=0.1",
                    "-vf", &format!("drawtext=text='{}':fontcolor=white:fontsize=120:x=(w-text_w)/2:y=(h-text_h)/2", escaped_word),
                    "-frames:v", "1",
                    "-y",
                    frame_path.to_str().unwrap(),
                ])
                .output()
                .context("Failed to generate frame")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("FFmpeg failed on word '{}': {}", word, stderr);
            }

            frame_number += 1;
        }
        print!(".");
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
    println!();

    // Create video from image sequence
    let pattern = temp_dir.join("frame_%06d.png");
    
    let output = Command::new("ffmpeg")
        .args([
            "-framerate", &args.fps.to_string(),
            "-i", pattern.to_str().unwrap(),
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-y",
            &args.output,
        ])
        .output()
        .context("Failed to create video from frames")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg video creation failed:\n{}", stderr);
    }

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();

    println!("✓ Video created: {}", args.output);
    Ok(())
}
