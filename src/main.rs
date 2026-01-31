use anyhow::{Context, Result};
use clap::Parser;
use os_info::Type;
use std::io::{self, IsTerminal};
use std::process::Command;
use std::time::Instant;

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
    #[arg(long, default_value = "true")]
    focus_lines: bool,

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

fn get_piped_input() -> Result<String> {
    let stdin = io::stdin();

    // 1. Check if the input is coming from a real person (keyboard)
    if stdin.is_terminal() {
        // If it's a terminal, the user likely didn't mean to pipe data.
        // You can return an error, show a help message, or skip reading.
        anyhow::bail!("No input detected via pipe. Usage: echo \"text\" | src-cli");
    }

    // 2. If we reach here, data is being piped in
    let mut buffer = String::new();
    // Use a scoped handle for better performance
    let mut handle = stdin.lock();
    io::Read::read_to_string(&mut handle, &mut buffer)?;

    // 3. Optional: error out if the pipe was empty
    if buffer.trim().is_empty() {
        anyhow::bail!("The piped input was empty.");
    }

    Ok(buffer)
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

    let mut font_location = args.font_location;
    // give font default location based on OS
    if font_location.len() == 0 {
        let info = os_info::get();
        match info.os_type() {
            Type::Debian => {
                println!("Running on Debian");
                font_location =
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string();
            }
            Type::Ubuntu => {
                println!("Running on Ubuntu");
                font_location =
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string();
            }
            Type::Linux => {
                println!("Running on a general Linux distribution.");
            }
            Type::Windows => {
                println!("Running on Windows");
            }
            _ => {
                println!("Running on a different OS: {:?}", info.os_type());
            }
        }
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
    let mut filters = Vec::new();

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
        let drawtext = format!(
            "drawtext=fontfile='{}':text='{}':fontcolor=white:fontsize={}:x=(w-text_w)/5*2:y=h/2-ascent:enable='between(t,{},{})'",
            font_location, escaped_word, fontsize, start_time, end_time
        );

        current_time = end_time;

        filters.push(drawtext);
    }

    // mark wpm
    let drawtext = format!(
        "drawtext=fontfile='{}':text='{} wpm':fontcolor={}:fontsize=60:x=(w-text_w)*0.9:y=(h-text_h)*0.9'",
        font_location, args.wpm, args.secondary_color
    );

    filters.push(drawtext);

    // Combine all filters
    let filter_chain = filters.join(",");

    println!("Rendering video...");

    let output = Command::new("ffmpeg")
        .args([
            "-hwaccel",
            "auto", // Use hardware acceleration if available
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={}:s=1920x1080:d={}", args.bg_color, total_duration),
            // add bgm for webm
            // ffmpeg -i video.mp4 -stream_loop -1 -i bgm.webm -map 0:v:0 -map 1:a:0 -c:v copy -c:a aac -b:a 192k -shortest
            "-stream_loop",
            "-1",
            "-i",
            &args.bgm_location,
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-shortest",
            // "-filter:a",
            // "\"loudnorm=I=-14:LRA=11:TP=-1.5:measured_I=-27.61:measured_LRA=18.06:measured_TP=-4.47:measured_thresh=-39.20:offset=0.58:linear=true\"",
            // end of add bgm
            "-vf",
            &filter_chain,
            "-c:v",
            "libx264",
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
