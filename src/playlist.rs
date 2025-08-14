use anyhow::{Result, anyhow};
use log::info;
use m3u8_rs::{Playlist, MediaPlaylist};
use reqwest::Client;
use std::sync::Arc;
use url::Url;
use hex;

#[derive(Debug, Clone)]
pub struct KeyInfo {
    pub method: String,
    pub uri: String,
    pub iv: Option<String>,
}

/// 获取并解析M3U8播放列表
pub async fn fetch_and_parse_playlist(client: Arc<Client>, url: Url) -> Result<(MediaPlaylist, Url, Option<KeyInfo>)> {
    info!("Fetching playlist from {}", url);
    
    let response = client.get(url.clone()).send().await?.error_for_status()?;
    let final_url = response.url().clone();
    let content = response.text().await?;

    let playlist = m3u8_rs::parse_playlist_res(content.as_bytes())
        .map_err(|e| anyhow!("Failed to parse M3U8 playlist: {}", e))?;

    match playlist {
        Playlist::MasterPlaylist(pl) => {
            info!("Master playlist found with {} variants.", pl.variants.len());
            
            let best_variant = pl.variants.iter()
                .max_by_key(|v| v.bandwidth)
                .ok_or_else(|| anyhow!("No variants found in master playlist"))?;
            
            info!("Selected variant with bandwidth: {}", best_variant.bandwidth);

            let media_playlist_url = final_url.join(&best_variant.uri)?;
            
            Box::pin(fetch_and_parse_playlist(client, media_playlist_url)).await
        }
        Playlist::MediaPlaylist(pl) => {
            info!("Media playlist found.");
            let key_info = pl.segments.iter().find_map(|s| s.key.as_ref()).map(|k| {
                let uri = k.uri.clone().unwrap_or_default();
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