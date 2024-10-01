#[cfg(target_os = "windows")]
extern crate vcpkg;

fn main() {
    #[cfg(target_os = "windows")]
    {
        setup_libpq_vcpkg();
    }
    tauri_build::build()
}

#[cfg(target_os = "windows")]
fn setup_libpq_vcpkg() {
    // Try to configure libpq using vcpkg
    vcpkg::probe_package("libpq")
        .map(|_| {
            // found libpq which depends on openssl
            vcpkg::Config::new().find_package("openssl").ok();

            println!("cargo:rustc-link-lib=crypt32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=secur32");
        })
        .is_ok()
}
