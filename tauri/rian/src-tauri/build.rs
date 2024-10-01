#[cfg(target_os = "windows")]
extern crate vcpkg;

fn main() {
    #[cfg(target_os = "windows")]
    {
        let pq_installed = setup_libpq_vcpkg();
        if !pq_installed {
            panic!("libpq not found");
        }
    }
    tauri_build::build()
}

#[cfg(target_os = "windows")]
fn setup_libpq_vcpkg() -> bool {
    vcpkg::find_package("libpq")
        .map(|_| {
            // found libpq which depends on openssl
            vcpkg::Config::new().find_package("openssl").ok();
            println!("cargo:rustc-link-lib=crypt32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=secur32");
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=wldap32");
        })
        .is_ok()
}
