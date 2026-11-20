fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_edition((1, 19))
        .with_style("fluent".into());
    if let Err(e) = slint_build::compile_with_config("ui/main.slint", config) {
        panic!("slint compilation failed: {e}");
    }
}
