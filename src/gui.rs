use crate::cli::Args;
use crate::run;
use anyhow::Result;
use egui::{Color32, RichText, Ui};
use egui_chinese_font::setup_chinese_fonts;
use poll_promise::Promise;
use rfd::FileDialog;
use std::path::PathBuf;

/// GUI应用状态
pub struct M3u8DownloaderApp {
    // 输入参数
    url: String,
    output_dir: String,
    output_video: String,
    threads: usize,
    ffmpeg_path: String,
    no_merge: bool,
    keep_segments: bool,
    headers: String,

    // 运行时状态
    download_promise: Option<Promise<Result<()>>>,
    status_message: String,
    status_color: Color32,
    is_downloading: bool,
}

impl Default for M3u8DownloaderApp {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_dir: "output".to_string(),
            output_video: "output_video.mp4".to_string(),
            threads: 10,
            ffmpeg_path: String::new(),
            no_merge: false,
            keep_segments: true,
            headers: String::new(),

            download_promise: None,
            status_message: "就绪".to_string(),
            status_color: Color32::GRAY,
            is_downloading: false,
        }
    }
}

impl M3u8DownloaderApp {
    /// 创建新的应用实例
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 设置默认主题
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.window_rounding = egui::Rounding::same(10.0);
        style.visuals.window_shadow.blur = 10.0;
        cc.egui_ctx.set_style(style);

        Self::default()
    }

    /// 选择输出目录
    fn select_output_dir(&mut self) {
        if let Some(path) = FileDialog::new().pick_folder() {
            self.output_dir = path.to_string_lossy().to_string();
        }
    }

    /// 选择FFmpeg路径
    fn select_ffmpeg_path(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("FFmpeg", &["exe"]).pick_file() {
            self.ffmpeg_path = path.to_string_lossy().to_string();
        }
    }

    /// 开始下载
    fn start_download(&mut self) {
        if self.url.is_empty() {
            self.status_message = "请输入 M3U8 URL".to_string();
            self.status_color = Color32::RED;
            return;
        }

        self.is_downloading = true;
        self.status_message = "下载中...".to_string();
        self.status_color = Color32::LIGHT_BLUE;

        // 解析HTTP头
        let headers = self
            .headers
            .split('\n')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();

        // 构建Args
        let args = Args {
            url: self.url.clone(),
            output_dir: PathBuf::from(&self.output_dir),
            output_video: self.output_video.clone(),
            threads: self.threads,
            ffmpeg_path: if self.ffmpeg_path.is_empty() {
                None
            } else {
                Some(PathBuf::from(&self.ffmpeg_path))
            },
            no_merge: self.no_merge,
            keep_segments: self.keep_segments,
            headers,
            gui: false, // 不需要在这里设置为true，因为已经在GUI模式中
        };

        // 在后台运行下载任务
        let args_clone = args.clone();
        self.download_promise = Some(Promise::spawn_thread("下载线程", move || {
            // 在新线程中创建一个tokio运行时
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async { run(args_clone).await })
        }));
    }

    /// 渲染输入表单
    fn render_input_form(&mut self, ui: &mut Ui) {
        ui.heading("M3U8 下载器");
        ui.add_space(10.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("input_grid")
                .num_columns(2)
                .spacing([20.0, 10.0])
                .striped(true)
                .show(ui, |ui| {
                    // URL输入
                    ui.label("M3U8 URL:");
                    ui.text_edit_singleline(&mut self.url);
                    ui.end_row();

                    // 输出目录
                    ui.label("输出目录:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.output_dir);
                        if ui.button("选择...").clicked() {
                            self.select_output_dir();
                        }
                    });
                    ui.end_row();

                    // 输出文件名
                    ui.label("输出文件名:");
                    ui.text_edit_singleline(&mut self.output_video);
                    ui.end_row();

                    // 线程数
                    ui.label("线程数:");
                    ui.add(egui::Slider::new(&mut self.threads, 1..=50));
                    ui.end_row();

                    // FFmpeg路径
                    ui.label("FFmpeg 路径 (可选):");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.ffmpeg_path);
                        if ui.button("选择...").clicked() {
                            self.select_ffmpeg_path();
                        }
                    });
                    ui.end_row();

                    // HTTP头
                    ui.label("custom HTTP headers (each line: Header: Value):");
                    ui.text_edit_multiline(&mut self.headers);
                    ui.end_row();

                    // 选项
                    ui.label("select:");
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.no_merge, "不合并视频");
                        ui.checkbox(&mut self.keep_segments, "保留分段文件");
                    });
                    ui.end_row();
                });
        });

        ui.add_space(15.0);

        // 下载按钮
        ui.vertical_centered_justified(|ui| {
            let download_button = egui::Button::new(RichText::new("开始下载").size(18.0))
                .min_size(egui::vec2(120.0, 30.0));

            if ui
                .add_enabled(!self.is_downloading, download_button)
                .clicked()
            {
                self.start_download();
            }
        });

        ui.add_space(10.0);

        // 状态信息
        ui.vertical_centered_justified(|ui| {
            ui.label(RichText::new(&self.status_message).color(self.status_color));
        });
    }

    /// 检查下载状态
    fn check_download_status(&mut self) {
        if let Some(promise) = &self.download_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(_) => {
                        self.status_message = "下载完成!".to_string();
                        self.status_color = Color32::GREEN;
                    }
                    Err(e) => {
                        self.status_message = format!("下载失败: {}", e);
                        self.status_color = Color32::RED;
                    }
                }
                self.is_downloading = false;
                self.download_promise = None;
            }
        }
    }
}

impl eframe::App for M3u8DownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 检查下载状态
        self.check_download_status();

        // 主窗口
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_input_form(ui);
            });
        });

        // 如果正在下载，持续重绘以更新状态
        if self.is_downloading {
            ctx.request_repaint();
        }
    }
}

/// 启动GUI应用
pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([670.0, 440.0])
            .with_min_inner_size([670.0, 440.0]),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "M3U8 Downloader",
        options,
        Box::new(|cc| {
            setup_chinese_fonts(&cc.egui_ctx).unwrap();
            Box::new(M3u8DownloaderApp::new(cc))
        }),
    )
}
