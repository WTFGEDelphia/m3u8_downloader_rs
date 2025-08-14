use clap::Parser;
use std::path::PathBuf;

/// A multi-threaded M3U8 downloader implemented in Rust.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The M3U8 URL to download.
    #[arg(short, long)]
    pub url: String,

    /// Directory to save the downloaded segments.
    #[arg(short, long, default_value = "output")]
    pub output_dir: PathBuf,

    /// Output video filename.
    #[arg(long, default_value = "output_video.mp4")]
    pub output_video: String,

    /// Maximum number of concurrent downloads.
    #[arg(short, long, default_value_t = 10)]
    pub threads: usize,

    /// Path to the FFmpeg executable.
    #[arg(long)]
    pub ffmpeg_path: Option<PathBuf>,

    /// Skip the merging step.
    #[arg(long)]
    pub no_merge: bool,

    /// Keep downloaded segments after merging.
    #[arg(long)]
    pub keep_segments: bool,

    /// Custom HTTP header(s). E.g., -H "Cookie: mycookie"
    #[arg(short = 'H', long = "header", action = clap::ArgAction::Append)]
    pub headers: Vec<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}