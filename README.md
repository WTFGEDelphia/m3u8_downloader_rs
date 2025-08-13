# M3U8 下载器 (Rust 实现)

一个高性能的多线程 M3U8 视频下载工具，使用 Rust 语言实现。支持 HLS 加密内容的解密和使用 FFmpeg 合并分段。

## 功能特点

- **多线程并发下载**：利用 Rust 的异步特性，支持并发下载分段，大幅提高下载速度
- **自动解析播放列表**：支持解析主播放列表和媒体播放列表
- **自动选择最佳质量**：从主播放列表中自动选择最高带宽的变体
- **AES-128 解密支持**：自动处理加密的 HLS 内容
- **FFmpeg 合并**：下载完成后自动使用 FFmpeg 合并分段为完整视频
- **进度显示**：实时显示下载进度
- **自定义 HTTP 头**：支持添加自定义 HTTP 头，如 Cookie、Referer 等
- **灵活的输出选项**：可选择是否保留原始分段文件

## 安装要求

- Rust 编译环境 (推荐 Rust 1.60+)
- FFmpeg (用于合并视频分段)

## 安装

```bash
# 克隆仓库
git clone https://github.com/WTFGEDelphia/m3u8_downloader_rs.git
cd m3u8_downloader_rs

# 编译
cargo build --release

# 安装到系统 (可选)
cargo install --path .
```

## 使用方法

### 基本用法

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" -o output_directory
```

### 命令行参数

```
USAGE:
    m3u8_downloader_rs [OPTIONS] --url <URL>

OPTIONS:
    -u, --url <URL>                     M3U8 URL 地址 (必需)
    -o, --output-dir <OUTPUT_DIR>       保存下载文件的目录 [默认: output]
    --output-video <OUTPUT_VIDEO>       输出视频文件名 [默认: output_video.mp4]
    -t, --threads <THREADS>             最大并发下载数 [默认: 10]
    --ffmpeg-path <FFMPEG_PATH>         FFmpeg 可执行文件路径 (可选，默认使用系统 PATH 中的 ffmpeg)
    --no-merge                          跳过合并步骤
    --keep-segments                     合并后保留分段文件
    -H, --header <HEADER>...            自定义 HTTP 头，例如: -H "Cookie: value" -H "Referer: url"
    -h, --help                          显示帮助信息
    -V, --version                       显示版本信息
```

### 示例

1. 使用 10 个线程下载 M3U8 视频：

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" -t 10
```

2. 指定输出目录和文件名：

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" -o my_videos --output-video my_movie.mp4
```

3. 添加自定义 HTTP 头：

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" -H "Cookie: session=abc123" -H "Referer: https://example.com"
```

4. 下载但不合并分段：

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" --no-merge
```

5. 合并后保留原始分段文件：

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" --keep-segments
```

6. 指定 FFmpeg 路径：

```bash
m3u8_downloader_rs -u "https://example.com/video.m3u8" --ffmpeg-path "C:\Program Files\FFmpeg\bin\ffmpeg.exe"
```

## 日志级别

可以通过设置环境变量 `RUST_LOG` 来控制日志输出级别：

```bash
# Windows
set RUST_LOG=info
m3u8_downloader_rs -u "https://example.com/video.m3u8"

# Linux/macOS
RUST_LOG=debug m3u8_downloader_rs -u "https://example.com/video.m3u8"
```

可用的日志级别：`error`, `warn`, `info`, `debug`, `trace`

## 技术细节

- 使用 `tokio` 进行异步操作
- 使用 `reqwest` 进行 HTTP 请求
- 使用 `m3u8-rs` 解析 M3U8 播放列表
- 使用 `aes` 和 `cbc` 进行 AES-128 解密
- 使用 `clap` 处理命令行参数
- 使用 `indicatif` 显示进度条

## 许可证

MIT