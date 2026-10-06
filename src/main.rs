mod audio;
mod tts;
mod ui;

use gpui_kit::component::Root;
use gpui_kit::*;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        "info,zbus=warn,tracing=warn,ureq=warn,hyper=warn,reqwest=warn,wgpu_hal=warn,wgpu_core=warn",
    ))
    .format_timestamp_millis()
    .init();
    log::info!("声谷 VoxVale 启动（本地 TTS · 数据不出设备）");
    log::info!("日志级别可通过 RUST_LOG 调整，如 RUST_LOG=voxvale=debug");

    gpui_kit::application()
        .run(|cx| {
            gpui_kit::init(cx);
            log::debug!("gpui 初始化完成");

            cx.spawn(async move |cx| {
                let _window = cx
                    .open_window(
                        WindowOptions {
                            window_min_size: Some(gpui_kit::size(px(520.0), px(760.0))),
                            titlebar: Some(TitlebarOptions {
                                title: Some("声谷 VoxVale".into()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        |window, cx| {
                            let view = cx.new(|cx| ui::VoxValeView::new(window, cx));
                            cx.new(|cx| Root::new(view, window, cx))
                        },
                    )
                    .expect("无法创建主窗口");
                log::info!("主窗口已打开");
            })
            .detach();
        });
}
