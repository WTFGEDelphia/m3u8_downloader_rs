use anyhow::{Result, anyhow};
use log::info;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// 合并下载的分段
pub async fn merge_segments(segments_dir: &Path, output_path: &String, ffmpeg_path: Option<&Path>, segment_count: usize) -> Result<()> {
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
    let status = Command::new(&ffmpeg)
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
        return Err(anyhow!("FFmpeg failed with exit code: {:?}", status.code()));
    }
    
    Ok(())
}

/// 清理下载的分段文件
pub async fn cleanup_segments(segments_dir: &Path) -> Result<()> {
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
        return Err(anyhow!("Failed to remove some files: {}", errors.join(", ")));
    }
    
    Ok(())
}