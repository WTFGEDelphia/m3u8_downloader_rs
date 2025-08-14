use log::{error, info};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 检查是否启动GUI模式（无参数）
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        // 无参数，直接启动GUI模式
        info!("Starting M3U8 downloader in GUI mode...");
        if let Err(e) = m3u8_downloader_rs::gui::run_gui() {
            error!("GUI error: {}", e);
            std::process::exit(1);
        }
    } else {
        // 解析命令行参数
        let cli_args = m3u8_downloader_rs::cli::parse_args();

        // 检查是否指定了--gui参数
        if cli_args.gui {
            // GUI模式
            info!("Starting M3U8 downloader in GUI mode...");
            if let Err(e) = m3u8_downloader_rs::gui::run_gui() {
                error!("GUI error: {}", e);
                std::process::exit(1);
            }
        } else {
            // 命令行模式
            info!("Starting M3U8 downloader in CLI mode...");
            info!("URL: {}", cli_args.url);

            // 运行下载器
            if let Err(e) = m3u8_downloader_rs::run(cli_args).await {
                error!("An error occurred: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
