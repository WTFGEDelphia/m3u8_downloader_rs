# M3U8 下载器 (Rust 实现)

一个高性能的多线程 M3U8 视频下载工具，使用 Rust 语言实现。支持 HLS 加密内容的解密和使用 FFmpeg 合并分段。采用模块化架构设计，代码结构清晰，易于维护和扩展。

## 功能特点

- **模块化架构**：采用 Rust 标准架构设计，代码组织清晰，易于维护和扩展
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

## 项目架构

项目采用模块化设计，遵循 Rust 标准架构实践，主要包含以下模块：

- **cli.rs**: 命令行参数处理模块，使用 `clap` 库定义和解析命令行参数
- **http.rs**: HTTP 客户端模块，处理网络请求和自定义请求头
- **playlist.rs**: M3U8 播放列表解析模块，处理主播放列表和媒体播放列表的解析
- **downloader.rs**: 下载模块，实现并发下载和进度显示
- **crypto.rs**: 解密模块，处理 AES-128 加密内容的解密
- **merger.rs**: 合并模块，使用 FFmpeg 合并下载的分段文件
- **lib.rs**: 库文件，整合各模块功能并提供主要运行逻辑
- **main.rs**: 主程序入口，负责初始化和调用库函数

这种模块化设计使代码结构更清晰，便于维护和扩展，同时提高了代码的可测试性。

## 技术细节

- 使用 `tokio` 进行异步操作和并发控制
- 使用 `reqwest` 进行 HTTP 请求
- 使用 `m3u8-rs` 解析 M3U8 播放列表
- 使用 `aes` 和 `cbc` 进行 AES-128 解密
- 使用 `clap` 处理命令行参数
- 使用 `indicatif` 显示进度条
- 使用 `anyhow` 进行错误处理
- 使用 `log` 和 `env_logger` 进行日志管理

## 开发与贡献

项目采用标准的 Rust 架构设计，欢迎贡献代码或提出改进建议。

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/WTFGEDelphia/m3u8_downloader_rs.git
cd m3u8_downloader_rs

# 安装开发依赖
cargo build

# 运行测试
cargo test
```

### 代码结构

```
src/
├── cli.rs       # 命令行参数处理
├── http.rs      # HTTP 客户端
├── playlist.rs  # M3U8 播放列表解析
├── downloader.rs # 下载功能
├── crypto.rs    # 解密功能
├── merger.rs    # 合并功能
├── lib.rs       # 库文件
└── main.rs      # 主程序入口
```

### 提交代码

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

MIT