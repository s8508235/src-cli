use std::io::{self, IsTerminal, Read};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use os_info::Type;

mod text_utils;
use text_utils::split_text;

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
    #[arg(long, default_value = "bgm.webm")]
    bgm_location: String,

    // local font location for output text
    #[arg(long, default_value = "")]
    font_location: String,
}

/// Validate FFmpeg color format
fn validate_color(color: &str) -> Result<()> {
    // FFmpeg supports: named colors, hex colors (#RRGGBB or 0xRRGGBB), and rgb(r,g,b)
    let color_lower = color.to_lowercase();

    // Check if it's a hex color
    if color.starts_with('#') || color.starts_with("0x") {
        let hex_part = color.trim_start_matches('#').trim_start_matches("0x");
        if hex_part.len() == 6 && hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(());
        }
        anyhow::bail!("Invalid hex color format. Use #RRGGBB or 0xRRGGBB (e.g., #FF0000)");
    }

    // Check if it's rgb() format
    if color_lower.starts_with("rgb(") && color_lower.ends_with(')') {
        return Ok(()); // Basic check, FFmpeg will validate the actual values
    }

    // Check if it's a named color (common FFmpeg/X11 color names)
    let valid_colors = [
        "black",
        "white",
        "red",
        "green",
        "blue",
        "yellow",
        "cyan",
        "magenta",
        "orange",
        "purple",
        "pink",
        "brown",
        "gray",
        "grey",
        "silver",
        "gold",
        "lime",
        "navy",
        "teal",
        "olive",
        "maroon",
        "aqua",
        "fuchsia",
        "darkred",
        "darkgreen",
        "darkblue",
        "lightred",
        "lightgreen",
        "lightblue",
        "darkgray",
        "darkgrey",
        "lightgray",
        "lightgrey",
        "dimgray",
        "dimgrey",
    ];

    if valid_colors.contains(&color_lower.as_str()) {
        return Ok(());
    }

    anyhow::bail!(
        "Invalid color '{}'. Use:\n  - Named colors (e.g., white, black, red, blue)\n  - Hex colors (e.g., #FF0000 or 0xFF0000)\n  - RGB format (e.g., rgb(255,0,0))",
        color
    );
}

fn get_piped_input() -> anyhow::Result<String> {
    #[cfg(windows)]
    println!("use cmd if encoding is wrong");

    let stdin = io::stdin();

    if stdin.is_terminal() {
        anyhow::bail!("No input detected via pipe. Usage: echo \"text\" | src-cli");
    }

    let mut buffer = Vec::new();
    let mut handle = stdin.lock();
    handle.read_to_end(&mut buffer)?;

    // Convert to String
    // from_utf8_lossy handles CJK characters correctly IF the source
    // is UTF-8 (which we will force in the next step).
    let content = String::from_utf8_lossy(&buffer).to_string();

    if content.trim().is_empty() {
        anyhow::bail!("The piped input was empty.");
    }

    Ok(content)
}

fn main() -> Result<()> {
    // Check if ffmpeg is available
    let ffmpeg_check = Command::new("ffmpeg").arg("-version").output();

    match ffmpeg_check {
        Ok(output) if output.status.success() => {
            // FFmpeg is available, continue
            let version_output = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = version_output.lines().next() {
                println!("FFmpeg found: {}", first_line);
            }
        }
        _ => {
            anyhow::bail!(
                "FFmpeg is not installed or not found in PATH. Please install FFmpeg first.\nVisit: https://ffmpeg.org/download.html"
            );
        }
    }

    let args = Args::parse();

    // in windows
    let mut font_location: String = args.font_location;

    // give font default location based on OS
    let info = os_info::get();
    if font_location.is_empty() {
        match info.os_type() {
            Type::Debian | Type::Ubuntu => {
                println!("Running on Debian/Ubuntu");

                // Linux: Try common CJK fonts
                let candidates = vec![
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                ];

                for font in candidates {
                    if std::path::Path::new(font).exists() {
                        font_location = font.to_string();
                        break;
                    }
                }
            }
            Type::Linux => {
                println!(
                    "Running on a general Linux distribution. You should provide your own font location"
                );
                std::process::exit(1);
            }
            Type::Windows => {
                // configurable font downgrade performance a lot on windows
                println!("Running on Windows, use msyh anyway");
            }
            Type::Macos => {
                font_location = "/System/Library/Fonts/PingFang.ttc".to_string();
                println!("Running on MacOS");
            }
            _ => {
                println!(
                    "Running on a different OS: {:?}, you should provide your own font location",
                    info.os_type()
                );
                std::process::exit(1);
            }
        }
    }

    println!("Using font {}", font_location);

    if !Path::new(&args.bgm_location).exists() {
        anyhow::bail!("BGM file not found at: {}", args.bgm_location);
        // Or if not using anyhow:
        // panic!("BGM file not found at: {}", args.bgm_location);
    }
    let bgm_check = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
        ])
        .arg(&args.bgm_location)
        .output();

    match bgm_check {
        Ok(output) if output.status.success() => {
            // bgm audio is available, continue
        }
        _ => {
            println!("bgm might be silent")
        }
    }

    // Validate colors
    validate_color(&args.text_color).context("Invalid text color")?;
    validate_color(&args.bg_color).context("Invalid background color")?;
    validate_color(&args.secondary_color).context("Invalid secondary line color")?;

    // Get input text from argument or stdin
    let text = match args.text {
        Some(t) => t,
        None => match get_piped_input() {
            Ok(text) => text,
            Err(e) => {
                // Prints the "No input detected" message and exits
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
    };

    if text.is_empty() {
        anyhow::bail!("No text provided. Use --text or pipe text to stdin.");
    }

    // Validation completed. Record the time before the operation
    let start = Instant::now();

    // Split text into words
    let words = split_text(&text);
    let word_count = words.len();

    // Calculate duration per word based on WPM
    let seconds_per_word = 60.0 / args.wpm as f64;
    let mut total_duration = seconds_per_word * word_count as f64;

    println!("Creating video: {}", args.output);
    println!(
        "Words: {} | WPM: {} | Duration per word: {:.2}s",
        word_count, args.wpm, seconds_per_word
    );

    // Build filter with multiple drawtext filters, each enabled for specific time range
    let mut filters: Vec<String> = Vec::new();

    // Add focus lines if enabled
    if args.focus_lines {
        // Top line
        filters.push(format!(
            "drawbox=x=0:y=ih*0.2:w=1920:h=10:t=fill:color={}",
            args.secondary_color
        ));

        // Bottom line
        filters.push(format!(
            "drawbox=x=0:y=ih*0.8:w=1920:h=10:t=fill:color={}",
            args.secondary_color
        ));

        // Left vertical line
        filters.push(format!(
            "drawbox=x=iw*0.4:y=ih*0.2:w=10:h=75:t=fill:color={}",
            args.secondary_color
        ));

        // Right vertical line
        filters.push(format!(
            "drawbox=x=iw*0.4:y=ih*0.8-75:w=10:h=75:t=fill:color={}",
            args.secondary_color
        ));
    }

    // Check if previous word ended a sentence (has punctuation)
    let mut current_time = 0.0;
    let mut last_relax_time = 0.0;
    for (i, word) in words.iter().enumerate() {
        let mut relax_time = 0.0;
        // relax every 60 second or ends with punctuation
        if i > 0
            && (last_relax_time > current_time + 60.0
                || word.ends_with('.')
                || word.ends_with('!')
                || word.ends_with('?'))
        {
            relax_time = args.rest_duration;
            total_duration += args.rest_duration;
            last_relax_time = current_time;
        }
        let start_time = current_time;
        let end_time = current_time + seconds_per_word + relax_time;

        // Escape word for FFmpeg
        let escaped_word = word
            .replace('\\', "\\\\")
            .replace('\'', "'\\''")
            .replace(':', "\\:");

        let mut fontsize = 100;
        if escaped_word.len() > 50 {
            fontsize = 80;
        }

        let drawtext = if info.os_type() == Type::Windows {
            format!(
                "drawtext=fontfile='C\\:/Windows/Fonts/msyh.ttc':text={}:fontcolor={}:fontsize={}:x=(w-text_w)/5*2:y=h/2-ascent:enable='between(t,{},{})'",
                escaped_word, args.text_color, fontsize, start_time, end_time
            )
        } else {
            format!(
                "drawtext=fontfile='{}':text='{}':fontcolor={}:fontsize={}:x=(w-text_w)/5*2:y=h/2-ascent:enable='between(t,{},{})'",
                font_location, escaped_word, args.text_color, fontsize, start_time, end_time
            )
        };

        current_time = end_time;

        filters.push(drawtext);
    }

    // mark wpm
    let drawtext = if info.os_type() == Type::Windows {
        println!("Running on Windows, use msyh despite of argument");
        format!(
            "drawtext=fontfile='C\\:/Windows/Fonts/msyh.ttc':text='{} wpm':fontcolor={}:fontsize=60:x=(w-text_w)*0.9:y=(h-text_h)*0.9",
            args.wpm, args.secondary_color
        )
    } else {
        format!(
            "drawtext=fontfile='{}':text='{} wpm':fontcolor={}:fontsize=60:x=(w-text_w)*0.9:y=(h-text_h)*0.9",
            font_location, args.wpm, args.secondary_color
        )
    };

    filters.push(drawtext);

    // Combine all filters
    let filter_chain = filters.join(",");

    println!("Rendering video...");

    let output = Command::new("ffmpeg")
        .env("FONTCONFIG_FILE", "NUL")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-hwaccel",
            "auto", // Use hardware acceleration if available
            "-f",
            "lavfi",
            "-i",
            &format!(
                "color=c={}:s=1920x1080:d={}:r=30",
                args.bg_color, total_duration
            ),
            // add bgm for webm
            // ffmpeg -i video.mp4 -stream_loop -1 -i bgm.webm -map 0:v:0 -map 1:a:0 -c:v copy -c:a aac -b:a 192k -shortest
            "-stream_loop",
            "-1",
            "-i",
            &args.bgm_location,
            // "-filter:a",
            // "\"loudnorm=I=-14:LRA=11:TP=-1.5:measured_I=-27.61:measured_LRA=18.06:measured_TP=-4.47:measured_thresh=-39.20:offset=0.58:linear=true\"",
            // end of add bgm
            "-vf",
            &filter_chain,
            "-map",
            "0:v:0", // Map video from input 0
            "-map",
            "1:a:0", // Map audio from input 1
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast", // Much faster encoding (was using default "medium")
            "-crf",
            "23", // Quality level (lower = better, 23 is good)
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-shortest",
            "-pix_fmt",
            "yuv420p",
            "-y",
            &args.output,
        ])
        .output()
        .context("Failed to execute ffmpeg. Make sure ffmpeg is installed and in PATH.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg failed:\n{}", stderr);
    }

    let duration = start.elapsed();
    println!(
        "✓ Video created: {} in {:.2}s with total {:.2}s",
        args.output,
        duration.as_secs_f64(),
        total_duration
    );
    Ok(())
}
