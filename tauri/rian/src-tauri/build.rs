fn main() {
    // This line adds the "+crt-static" flag to the rustc compiler when building for Windows.
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-cfg=target_feature=\"crt-static\"");
    }
    tauri_build::build()
}
