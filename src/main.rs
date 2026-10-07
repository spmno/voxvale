mod audio;
mod i18n;
mod tts;
mod ui;

use gpui_kit::component::Root;
use gpui_kit::*;
use crate::i18n::t;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        "info,zbus=warn,tracing=warn,ureq=warn,hyper=warn,reqwest=warn,wgpu_hal=warn,wgpu_core=warn",
    ))
    .format_timestamp_millis()
    .init();
    log::info!("{}", t("log_startup"));
    log::info!("{}", t("log_log_level"));

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            log::debug!("{}", t("log_gpui_init"));

            cx.spawn(async move |cx| {
                let _window = cx
                    .open_window(
                        WindowOptions {
                            window_min_size: Some(gpui_kit::size(px(520.0), px(760.0))),
                            titlebar: Some(TitlebarOptions {
                                title: Some(t("window_title").into()),
                                ..gpui_kit::component::TitleBar::title_bar_options()
                            }),
                            // 强制客户端装饰：GNOME Wayland 的服务端装饰默认只有关闭键，
                            // 客户端模式让 TitleBar 自绘最小化/最大化/关闭
                            window_decorations: Some(WindowDecorations::Client),
                            ..gpui_kit::component::TitleBar::window_options()
                        },
                        |window, cx| {
                            let view = cx.new(|cx| ui::VoxValeView::new(window, cx));
                            cx.new(|cx| Root::new(view, window, cx))
                        },
                    )
                    .expect("无法创建主窗口");
                log::info!("{}", t("log_window_opened"));
            })
            .detach();
        });
}
