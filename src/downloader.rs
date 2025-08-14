use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use m3u8_rs::MediaSegment;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::crypto::decrypt_data;
use crate::playlist::KeyInfo;

/// 下载所有分段
pub async fn download_segments(
    client: Arc<Client>,
    segments: &[MediaSegment],
    base_url: Url,
    output_dir: PathBuf,
    max_concurrency: usize,
    key_info: Option<KeyInfo>,
) -> Vec<Result<()>> {
    let pb = Arc::new(ProgressBar::new(segments.len() as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    // 收集所有分段信息，避免在异步闭包中使用引用
    let mut segments_info = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        let segment_uri = segment.uri.clone();
        let segment_url = match base_url.join(&segment_uri) {
            Ok(url) => url,
            Err(e) => {
                return vec![Err(anyhow!(
                    "无法解析分段URL: {} - 错误: {}",
                    segment_uri,
                    e
                ))];
            }
        };
        let output_path = output_dir.join(format!("index{}.ts", i));
        segments_info.push((i, segment_url, output_path));
    }

    let base_url_clone = base_url.clone();

    let fetches = stream::iter(segments_info)
        .map(|(i, segment_url, output_path)| {
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
                                Err(e) => {
                                    return Err(anyhow!(
                                        "无法解析密钥URL: {} - 错误: {}",
                                        ki.uri,
                                        e
                                    ))
                                }
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
                        Err(e) => return Err(anyhow!("无法解析IV值: {} - 错误: {}", iv_str, e)),
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

                match download_segment(
                    client.clone(),
                    &segment_url,
                    &output_path,
                    key.as_deref(),
                    iv.as_deref(),
                )
                .await
                {
                    Ok(_) => {
                        pb_clone.inc(1);
                        Ok(())
                    }
                    Err(e) => {
                        pb_clone.inc(1);
                        Err(anyhow!("Failed to download {}: {}", segment_url, e))
                    }
                }
            })
        })
        .buffer_unordered(max_concurrency);

    let results: Vec<_> = fetches.collect().await;
    pb.finish_with_message("downloaded");

    results
        .into_iter()
        .map(|res| match res {
            Ok(inner_res) => inner_res,
            Err(e) => Err(anyhow!("Tokio task failed: {}", e)),
        })
        .collect()
}

/// 下载单个分段
async fn download_segment(
    client: Arc<Client>,
    url: &Url,
    path: &Path,
    key: Option<&[u8]>,
    iv: Option<&[u8]>,
) -> Result<()> {
    let mut response = client.get(url.clone()).send().await?.error_for_status()?;
    let mut encrypted_data = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        encrypted_data.extend_from_slice(&chunk);
    }

    let decrypted_data = if let (Some(key), Some(iv)) = (key, iv) {
        decrypt_data(&encrypted_data, key, iv)?
    } else {
        encrypted_data
    };

    let mut file = fs::File::create(path).await?;
    file.write_all(&decrypted_data).await?;

    Ok(())
}
