use log::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 解析命令行参数
    let args = m3u8_downloader_rs::cli::parse_args();

    info!("Starting M3U8 downloader...");
    info!("URL: {}", args.url);

    // 运行下载器
    if let Err(e) = m3u8_downloader_rs::run(args).await {
        error!("An error occurred: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
