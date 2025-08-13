use clap::Parser;
use log::{info, error, warn, debug};
use std::path::{Path, PathBuf};
use std::time::Duration;
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use url::Url;
use m3u8_rs::{Playlist, MediaPlaylist, MediaSegment};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use hex;

/// A multi-threaded M3U8 downloader implemented in Rust.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The M3U8 URL to download.
    #[arg(short, long)]
    url: String,

    /// Directory to save the downloaded segments.
    #[arg(short, long, default_value = "output")]
    output_dir: PathBuf,

    /// Output video filename.
    #[arg(long, default_value = "output_video.mp4")]
    output_video: String,

    /// Maximum number of concurrent downloads.
    #[arg(short, long, default_value_t = 10)]
    threads: usize,

    /// Path to the FFmpeg executable.
    #[arg(long)]
    ffmpeg_path: Option<PathBuf>,

    /// Skip the merging step.
    #[arg(long)]
    no_merge: bool,

    /// Keep downloaded segments after merging.
    #[arg(long)]
    keep_segments: bool,

    /// Custom HTTP header(s). E.g., -H "Cookie: mycookie"
    #[arg(short = 'H', long = "header", action = clap::ArgAction::Append)]
    headers: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    info!("Starting M3U8 downloader...");
    info!("URL: {}", args.url);
    
    if let Err(e) = run(args).await {
        error!("An error occurred: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct KeyInfo {
    method: String,
    uri: String,
    iv: Option<String>,
}

async fn run(args: Args) -> anyhow::Result<()> {
    let client = Arc::new(build_http_client(&args.headers)?);
    let m3u8_url = Url::parse(&args.url)?;

    // Create a unique output directory for this URL to avoid conflicts
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
        // let output_video_path = args.output_dir.join(&args.output_video);
        let output_video_path = &args.output_video;
        info!("Merging segments into: {:?}", output_video_path);
        
        match merge_segments(&output_dir, &output_video_path, args.ffmpeg_path.as_deref(), media_playlist.segments.len()).await {
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
                Err(e) => warn!("Failed to clean up some segment files: {}", e),
            }
        }
    } else {
        info!("Skipping merge step as requested.");
    }

    Ok(())
}

async fn download_segments(
    client: Arc<Client>,
    segments: &[MediaSegment],
    base_url: Url,  // 改为拥有所有权
    output_dir: PathBuf,  // 改为拥有所有权
    max_concurrency: usize,
    key_info: Option<KeyInfo>,  // 改为拥有所有权
) -> Vec<anyhow::Result<()>> {
    let pb = Arc::new(ProgressBar::new(segments.len() as u64));
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    // 收集所有分段信息，避免在异步闭包中使用引用
    let mut segments_info = Vec::new();
    
    for (i, segment) in segments.iter().enumerate() {
        let segment_uri = segment.uri.clone();
        let segment_url = match base_url.join(&segment_uri) {
            Ok(url) => url,
            Err(e) => {
                return vec![Err(anyhow::anyhow!("无法解析分段URL: {} - 错误: {}", segment_uri, e))];
            }
        };
        let output_path = output_dir.join(format!("index{}.ts", i));
        segments_info.push((i, segment_uri, segment_url, output_path));
    }
    
    let base_url_clone = base_url.clone();
    
    let fetches = stream::iter(segments_info)
        .map(|(i, segment_uri, segment_url, output_path)| {
            let client = client.clone();
            let pb_clone = pb.clone();
            let key_info_clone = key_info.clone();
            let base_url = base_url_clone.clone();

            tokio::spawn(async move {
                if fs::metadata(&output_path).await.is_ok() {
                    debug!("Segment {:?} already exists. Skipping.", output_path);
                    pb_clone.inc(1);
                    return Ok(());
                }

                let (key, iv) = if let Some(ki) = key_info_clone {
                    let key_url = match Url::parse(&ki.uri) {
                        Ok(url) => url,
                        Err(_) => {
                            // 尝试将key URI作为相对URL处理
                            match base_url.join(&ki.uri) {
                                Ok(url) => url,
                                Err(e) => return Err(anyhow::anyhow!("无法解析密钥URL: {} - 错误: {}", ki.uri, e))
                            }
                        }
                    };
                    let mut key_bytes = client.get(key_url).send().await?.bytes().await?.to_vec();
                    // 确保密钥长度为16字节（AES-128要求）
                    if key_bytes.len() > 16 {
                        key_bytes.truncate(16);
                    } else if key_bytes.len() < 16 {
                        // 如果密钥长度不足16字节，用0填充
                        key_bytes.resize(16, 0);
                    }
                    let iv_str = ki.iv.clone().unwrap_or_else(|| format!("0x{:032x}", i));
                    let mut iv_bytes = match hex::decode(iv_str.trim_start_matches("0x")) {
                        Ok(bytes) => bytes,
                        Err(e) => return Err(anyhow::anyhow!("无法解析IV值: {} - 错误: {}", iv_str, e))
                    };
                    
                    // 确保IV长度为16字节（AES-128要求）
                    if iv_bytes.len() > 16 {
                        iv_bytes.truncate(16);
                    } else if iv_bytes.len() < 16 {
                        // 如果IV长度不足16字节，用0填充
                        iv_bytes.resize(16, 0);
                    }
                    (Some(key_bytes), Some(iv_bytes))
                } else {
                    (None, None)
                };

                match download_segment(client.clone(), &segment_url, &output_path, key.as_deref(), iv.as_deref()).await {
                    Ok(_) => {
                        pb_clone.inc(1);
                        Ok(())
                    }
                    Err(e) => {
                        pb_clone.inc(1);
                        Err(anyhow::anyhow!("Failed to download {}: {}", segment_url, e))
                    }
                }
            })
        })
        .buffer_unordered(max_concurrency);

    let results: Vec<_> = fetches.collect().await;
    pb.finish_with_message("downloaded");

    results.into_iter().map(|res| match res {
        Ok(inner_res) => inner_res,
        Err(e) => Err(anyhow::anyhow!("Tokio task failed: {}", e)),
    }).collect()
}

async fn download_segment(client: Arc<Client>, url: &Url, path: &Path, key: Option<&[u8]>, iv: Option<&[u8]>) -> anyhow::Result<()> {
    let mut response = client.get(url.clone()).send().await?.error_for_status()?;
    let mut encrypted_data = Vec::new();
    
    while let Some(chunk) = response.chunk().await? {
        encrypted_data.extend_from_slice(&chunk);
    }
    
    let decrypted_data = if let (Some(key), Some(iv)) = (key, iv) {
        use aes::cipher::{BlockDecryptMut, KeyIvInit};
        use cbc::Decryptor;
        use aes::cipher::block_padding::Pkcs7;
        
        let cipher = Decryptor::<aes::Aes128>::new(key.into(), iv.into());
        let mut buf = encrypted_data.to_vec();
        let decrypted_slice = cipher.decrypt_padded_mut::<Pkcs7>(&mut buf).map_err(|e| anyhow::anyhow!("Decryption error: {}", e))?;
        decrypted_slice.to_vec()
    } else {
        encrypted_data
    };
    
    let mut file = fs::File::create(path).await?;
    file.write_all(&decrypted_data).await?;
    
    Ok(())
}


async fn fetch_and_parse_playlist(client: Arc<Client>, url: Url) -> anyhow::Result<(MediaPlaylist, Url, Option<KeyInfo>)> {
    info!("Fetching playlist from {}", url);
    
    let response = client.get(url.clone()).send().await?.error_for_status()?;
    let final_url = response.url().clone();
    let content = response.text().await?;

    let playlist = m3u8_rs::parse_playlist_res(content.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to parse M3U8 playlist: {}", e))?;

    match playlist {
        Playlist::MasterPlaylist(pl) => {
            info!("Master playlist found with {} variants.", pl.variants.len());
            
            let best_variant = pl.variants.iter()
                .max_by_key(|v| v.bandwidth)
                .ok_or_else(|| anyhow::anyhow!("No variants found in master playlist"))?;
            
            info!("Selected variant with bandwidth: {}", best_variant.bandwidth);

            let media_playlist_url = final_url.join(&best_variant.uri)?;
            
            Box::pin(fetch_and_parse_playlist(client, media_playlist_url)).await
        }
        Playlist::MediaPlaylist(pl) => {
            info!("Media playlist found.");
            let key_info = pl.segments.iter().find_map(|s| s.key.as_ref()).map(|k| {
                let uri = k.uri.clone().unwrap_or_default();
                // 不在这里处理相对URL，而是在实际使用时处理
                // 因为这里我们已经有了final_url作为基础URL
                KeyInfo {
                    method: k.method.to_string(),
                    uri,
                    iv: k.iv.as_ref().map(|i| hex::encode(i)),
                }
            });
            Ok((pl, final_url, key_info))
        }
    }
}

fn build_http_client(custom_headers: &[String]) -> anyhow::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36"));

    for header in custom_headers {
        if let Some((key, value)) = header.split_once(':') {
            let header_name = HeaderName::from_bytes(key.trim().as_bytes())?;
            let header_value = HeaderValue::from_str(value.trim())?;
            headers.insert(header_name, header_value);
        } else {
            warn!("Ignoring malformed header: {}", header);
        }
    }
    
    debug!("Using HTTP headers: {:?}", headers);

    let client = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    Ok(client)
}

async fn merge_segments(segments_dir: &Path, output_path: &String, ffmpeg_path: Option<&Path>, segment_count: usize) -> anyhow::Result<()> {
    // 创建一个临时文件列表
    let file_list_path = segments_dir.join("filelist.txt");
    let mut file_list = fs::File::create(&file_list_path).await?;
    
    // 写入文件列表
    for i in 0..segment_count {
        let segment_path = format!("index{}.ts", i);
        file_list.write_all(format!("file '{}'", segment_path).as_bytes()).await?;
        file_list.write_all(b"\n").await?;
    }
    file_list.flush().await?;
    
    // 确定ffmpeg路径
    let ffmpeg = match ffmpeg_path {
        Some(path) => path.to_path_buf(),
        None => PathBuf::from("ffmpeg"), // 默认使用系统PATH中的ffmpeg
    };
    
    // 构建ffmpeg命令
    let status = tokio::process::Command::new(&ffmpeg)
        .current_dir(segments_dir) // 设置工作目录为分段目录
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg("filelist.txt")
        .arg("-c")
        .arg("copy")
        .arg("-bsf:a")
        .arg("aac_adtstoasc")
        .arg("-movflags")
        .arg("+faststart")
        .arg("-y")
        .arg(output_path)
        .status()
        .await?;
    
    // 删除临时文件列表
    let _ = fs::remove_file(&file_list_path).await;
    
    if !status.success() {
        return Err(anyhow::anyhow!("FFmpeg failed with exit code: {:?}", status.code()));
    }
    
    Ok(())
}

async fn cleanup_segments(segments_dir: &Path) -> anyhow::Result<()> {
    let mut read_dir = fs::read_dir(segments_dir).await?;
    let mut errors = Vec::new();
    
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "ts" {
                if let Err(e) = fs::remove_file(&path).await {
                    errors.push(format!("Failed to remove {:?}: {}", path, e));
                }
            }
        }
    }
    
    if !errors.is_empty() {
        return Err(anyhow::anyhow!("Failed to remove some files: {}", errors.join(", ")));
    }
    
    Ok(())
}
