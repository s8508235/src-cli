use std::io::{self, IsTerminal, Read};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use os_info::Type;

mod text;
use text::split_text;

pub fn check_ffmpeg() -> Result<()> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("Failed to execute ffmpeg command")?;

    if !output.status.success() {
        bail!(
            "FFmpeg is not installed or not found in PATH. Please install FFmpeg first.\nVisit: https://ffmpeg.org/download.html"
        );
    }

    // Use idiomatic code structure
    let version_output = String::from_utf8_lossy(&output.stdout);
    if let Some(first_line) = version_output.lines().next() {
        println!("FFmpeg found: {}", first_line);
    }

    Ok(())
}
// Validate FFmpeg color format
fn validate_color(color: &str) -> Result<()> {
    let color_lower = color.to_lowercase();

    // Check hex colors
    if color.starts_with('#') || color.starts_with("0x") {
        let hex_part = color.trim_start_matches('#').trim_start_matches("0x");
        if hex_part.len() == 6 && hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(());
        }
        bail!("Invalid hex color format. Use #RRGGBB or 0xRRGGBB (e.g., #FF0000)");
    }

    // Check RGB format
    if color_lower.starts_with("rgb(") && color_lower.ends_with(')') {
        return Ok(());
    }

    const VALID_COLORS: &[&str] = &[
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

    if VALID_COLORS.contains(&color_lower.as_str()) {
        return Ok(());
    }

    bail!(
        "Invalid color '{}'. Use:\n  - Named colors (e.g., white, black, red, blue)\n  - Hex colors (e.g., #FF0000 or 0xFF0000)\n  - RGB format (e.g., rgb(255,0,0))",
        color
    );
}

fn get_piped_input() -> anyhow::Result<String> {
    #[cfg(windows)]
    println!("use cmd if encoding is wrong");

    let stdin = io::stdin();

    if stdin.is_terminal() {
        bail!("No input detected via pipe. Usage: echo \"text\" | src-cli");
    }

    let mut buffer = Vec::new();
    stdin
        .lock()
        .read_to_end(&mut buffer)
        .context("Failed to read from stdin")?;

    let content = String::from_utf8_lossy(&buffer).to_string();

    if content.trim().is_empty() {
        bail!("The piped input was empty.");
    }

    Ok(content)
}

// Configuration for font selection based on OS
struct FontConfig {}

impl FontConfig {
    // Get default font location based on OS
    fn get_default_font() -> Result<String> {
        let info = os_info::get();

        match info.os_type() {
            Type::Debian | Type::Ubuntu => {
                println!("Running on Debian/Ubuntu");
                Self::find_linux_font()
            }
            Type::Linux => {
                bail!(
                    "Running on a general Linux distribution. Please provide font location via --font-location"
                )
            }
            Type::Windows => {
                println!("Running on Windows");
                Ok(Self::get_windows_font())
            }
            Type::Macos => {
                println!("Running on MacOS");
                Self::find_macos_font()
            }
            _ => {
                bail!(
                    "Unsupported OS: {:?}. Please provide font location via --font-location",
                    info.os_type()
                )
            }
        }
    }

    fn find_linux_font() -> Result<String> {
        const CANDIDATES: &[&str] = &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ];

        CANDIDATES
            .iter()
            .find(|&&font| Path::new(font).exists())
            .map(|&font| font.to_string())
            .context("No suitable CJK font found on Linux system")
    }

    fn get_windows_font() -> String {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        format!("{}/Fonts/msyh.ttc", windir.replace("\\", "/")).replace(":", "\\:")
    }

    fn find_macos_font() -> Result<String> {
        const CANDIDATES: &[&str] = &[
            "/Library/Fonts/Arial Unicode.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ];

        CANDIDATES
            .iter()
            .find(|&&font| Path::new(font).exists())
            .map(|&font| font.to_string())
            .context("No suitable CJK font found on MacOS")
    }
}

// Validate and prepare BGM file
fn validate_bgm(bgm_path: Option<String>) -> Result<Option<String>> {
    let Some(path) = bgm_path else {
        println!("No BGM provided");
        return Ok(None);
    };

    if !Path::new(&path).exists() {
        println!("BGM file not found at: '{}', processing with no bgm", path);
        return Ok(None);
    }

    // Verify audio stream exists
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
        ])
        .arg(&path)
        .output()
        .context("Failed to run ffprobe. Is it installed?")?;

    if !output.status.success() {
        println!("Warning: Could not verify BGM audio stream");
        return Ok(Some(path));
    }

    let streams = String::from_utf8_lossy(&output.stdout);
    if !streams.contains("audio") {
        bail!("BGM file has no audio stream: {}", path);
    }

    println!("BGM found and validated: {}", path);
    Ok(Some(path))
}

// Build drawtext filter for a single word
fn build_word_filter(
    word: &str,
    font_location: &str,
    text_color: &str,
    start_time: f64,
    end_time: f64,
) -> String {
    let escaped_word = word
        .replace('\\', "\\\\")
        .replace('\'', "'\\''")
        .replace(':', "\\:");

    let fontsize = if escaped_word.len() > 50 { 80 } else { 100 };

    format!(
        "drawtext=fontfile='{}':text='{}':fontcolor={}:fontsize={}:x=(w-text_w)/5*2:y=h/2-ascent:enable='between(t,{},{})'",
        font_location, escaped_word, text_color, fontsize, start_time, end_time
    )
}

// Build all video filters
fn build_filters(
    words: &[String],
    wpm: u32,
    text_color: &str,
    secondary_color: &str,
    rest_duration: f64,
    focus_lines: bool,
    font_location: &str,
) -> (Vec<String>, f64) {
    let seconds_per_word = 60.0 / wpm as f64;
    let mut total_duration = seconds_per_word * (words.len() as f64);

    // Use with_capacity when size is known
    let mut filters = Vec::with_capacity(words.len() + 5);

    // Add focus lines
    if focus_lines {
        filters.extend([
            format!(
                "drawbox=x=0:y=ih*0.2:w=1920:h=10:t=fill:color={}",
                secondary_color
            ),
            format!(
                "drawbox=x=0:y=ih*0.8:w=1920:h=10:t=fill:color={}",
                secondary_color
            ),
            format!(
                "drawbox=x=iw*0.4:y=ih*0.2:w=10:h=75:t=fill:color={}",
                secondary_color
            ),
            format!(
                "drawbox=x=iw*0.4:y=ih*0.8-75:w=10:h=75:t=fill:color={}",
                secondary_color
            ),
        ]);
    }

    // Add word filters with rest periods
    let mut current_time = 0.0;
    let mut last_relax_time = 0.0;

    for (i, word) in words.iter().enumerate() {
        let needs_rest = i > 0
            && (last_relax_time > current_time + 60.0
                || word.ends_with('.')
                || word.ends_with('!')
                || word.ends_with('?'));

        let relax_time = if needs_rest {
            last_relax_time = current_time;
            total_duration += rest_duration;
            rest_duration
        } else {
            0.0
        };

        let start_time = current_time;
        let end_time = current_time + seconds_per_word + relax_time;

        filters.push(build_word_filter(
            word,
            font_location,
            text_color,
            start_time,
            end_time,
        ));

        current_time = end_time;
    }

    // Add WPM indicator
    filters.push(format!(
        "drawtext=fontfile='{}':text='{} wpm':fontcolor={}:fontsize=60:x=(w-text_w)*0.9:y=(h-text_h)*0.9",
        font_location, wpm, secondary_color
    ));

    (filters, total_duration)
}

// Build FFmpeg command
fn build_ffmpeg_command(
    output_file: &str,
    bg_color: &str,
    bgm_location: Option<&str>,
    filter_chain: &str,
    total_duration: f64,
    overwrite: bool,
) -> Command {
    let mut cmd = Command::new("ffmpeg");

    cmd.env("FONTCONFIG_FILE", "NUL").args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-hwaccel",
        "auto",
        "-f",
        "lavfi",
        "-i",
        &format!("color=c={}:s=1920x1080:d={}:r=30", bg_color, total_duration),
    ]);

    // Add BGM if present
    if let Some(bgm) = bgm_location {
        cmd.args(["-stream_loop", "-1", "-i", bgm]);
    }

    // Video filter and stream mapping
    cmd.args(["-vf", filter_chain]);

    if bgm_location.is_some() {
        cmd.args(["-map", "0:v:0", "-map", "1:a:0"]);
    } else {
        cmd.args(["-map", "0:v:0"]);
    }

    // Codec settings
    cmd.args([
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-crf",
        "23",
        "-pix_fmt",
        "yuv420p",
    ]);

    if bgm_location.is_some() {
        cmd.args(["-c:a", "aac", "-b:a", "192k", "-shortest"]);
    }

    // Overwrite flag
    if overwrite {
        cmd.arg("-y");
    }

    cmd.arg(output_file);
    cmd
}

pub fn generate_video(args: crate::Args) -> Result<()> {
    let start = Instant::now();
    // Extract owned values that will be moved
    let text_opt = args.text;
    let bgm_opt = args.bgm_location;
    let font_opt = args.font_location;

    // Get font location
    let font_location = font_opt
        .or_else(|| FontConfig::get_default_font().ok())
        .context("No font available. Provide --font-location")?;

    println!("Using font: {}", font_location);

    // Validate BGM (takes ownership)
    let bgm_location = validate_bgm(bgm_opt)?;

    // Validate colors
    validate_color(&args.text_color).context("Invalid text color")?;
    validate_color(&args.bg_color).context("Invalid background color")?;
    validate_color(&args.secondary_color).context("Invalid secondary color")?;

    // Get input text from argument or stdin

    let text = text_opt.map(Ok).unwrap_or_else(get_piped_input)?;

    // Process words
    let words = split_text(&text);
    let word_count = words.len();
    let seconds_per_word = 60.0 / args.wpm as f64;

    println!("Creating video: {}", args.output);
    println!(
        "Words: {} | WPM: {} | Duration per word: {:.2}s",
        word_count, args.wpm, seconds_per_word
    );

    // Build filters
    let (filters, total_duration) = build_filters(
        &words,
        args.wpm,
        &args.text_color,
        &args.secondary_color,
        args.rest_duration,
        args.focus_lines,
        &font_location,
    );
    let filter_chain = filters.join(",");

    println!("Rendering video...");

    // Execute FFmpeg
    let mut cmd = build_ffmpeg_command(
        &args.output,
        &args.bg_color,
        bgm_location.as_deref(),
        &filter_chain,
        total_duration,
        args.overwrite_output_file.unwrap_or(false),
    );
    let output = cmd
        .output()
        .context("Failed to execute ffmpeg. Is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("FFmpeg failed:\n{}", stderr);
    }

    let duration = start.elapsed();
    println!(
        "✓ Video created: {} in {:.2}s (total video: {:.2}s)",
        args.output,
        duration.as_secs_f64(),
        total_duration
    );

    Ok(())
}
