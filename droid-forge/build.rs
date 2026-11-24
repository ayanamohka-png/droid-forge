fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("material".into());

    slint_build::compile_with_config("ui/app.slint", config)
        .expect("Failed to compile Slint UI");

    #[cfg(windows)]
    {
        if std::path::Path::new("assets/icon.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            let _ = res.compile();
        }
    }
}
