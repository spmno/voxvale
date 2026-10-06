use gpui_kit::AssetSource;

#[test]
fn asset_source_serves_all_ui_icons() {
    let paths = [
        // 标题栏窗口控制按钮
        "icons/window-minimize.svg",
        "icons/window-maximize.svg",
        "icons/window-close.svg",
        // 界面图标（与 ui.rs 引用一一对应）
        "icons/mic.svg",
        "icons/copy.svg",
        "icons/user.svg",
        "icons/book-open.svg",
        "icons/settings-2.svg",
        "icons/folder-open.svg",
        "icons/file.svg",
        "icons/play.svg",
        "icons/square.svg",
        "icons/loader-circle.svg",
        "icons/circle-check.svg",
        "icons/circle-x.svg",
        "icons/info.svg",
    ];
    for path in paths {
        let data = gpui_kit::assets::Assets
            .load(path)
            .expect("资源源返回错误")
            .unwrap_or_else(|| panic!("资源源未包含 {path}"));
        assert!(!data.is_empty(), "{path} 内容为空");
    }
}
