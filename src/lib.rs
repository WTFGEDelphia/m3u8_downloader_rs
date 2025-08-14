pub mod cli;
pub mod http;
pub mod playlist;
pub mod downloader;
pub mod crypto;
pub mod merger;

use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use tokio::fs;
use url::Url;

use crate::cli::Args;
use crate::http::build_http_client;
use crate::playlist::fetch_and_parse_playlist;
use crate::downloader::download_segments;
use crate::merger::{merge_segments, cleanup_segments};

/// 运行M3U8下载器的主要逻辑
pub async fn run(args: Args) -> Result<()> {
    let client = Arc::new(build_http_client(&args.headers)?);
    let m3u8_url = Url::parse(&args.url)?;

    // 创建一个唯一的输出目录，避免冲突
    let url_hash = &sha256::digest(&args.url)[..12];
    let output_dir = args.output_dir.join(url_hash);
    info!("Segments will be saved to: {:?}", output_dir);
    fs::create_dir_all(&output_dir).await?;

    let (media_playlist, base_url, key_info) = fetch_and_parse_playlist(client.clone(), m3u8_url).await?;
    
    info!("Successfully parsed media playlist. Found {} segments.", media_playlist.segments.len());

    let download_results = download_segments(
        client,
        &media_playlist.segments,
        base_url,
        output_dir.clone(),
        args.threads,
        key_info,
    ).await;

    let successful_downloads = download_results.iter().filter(|&r| r.is_ok()).count();
    let failed_downloads = download_results.len() - successful_downloads;

    if failed_downloads > 0 {
        error!("Failed to download {} out of {} segments.", failed_downloads, media_playlist.segments.len());
        for result in download_results {
            if let Err(e) = result {
                error!(" - {}", e);
            }
        }
        anyhow::bail!("Download failed for some segments. Aborting.");
    }

    info!("All {} segments downloaded successfully.", successful_downloads);

    // 合并文件
    if !args.no_merge {
        let output_video_path = &args.output_video;
        info!("Merging segments into: {:?}", output_video_path);
        
        match merge_segments(&output_dir, output_video_path, args.ffmpeg_path.as_deref(), media_playlist.segments.len()).await {
            Ok(_) => info!("Successfully merged segments into {:?}", output_video_path),
            Err(e) => {
                error!("Failed to merge segments: {}", e);
                anyhow::bail!("Merging failed. Segments are still available in {:?}", output_dir);
            }
        }
        
        // 清理分段文件
        if !args.keep_segments {
            info!("Cleaning up segment files...");
            match cleanup_segments(&output_dir).await {
                Ok(_) => info!("Segment files cleaned up successfully."),
                Err(e) => error!("Failed to clean up some segment files: {}", e),
            }
        }
    } else {
        info!("Skipping merge step as requested.");
    }

    Ok(())
}