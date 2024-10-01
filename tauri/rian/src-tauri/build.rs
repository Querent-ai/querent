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
    match vcpkg::find_package("libpq") {
        Ok(_) => {
            // vcpkg setup succeeded
            println!("cargo:rustc-link-lib=libpq");
            println!("cargo:rustc-link-lib=crypt32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=secur32");
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=wldap32");
        }
        Err(_) => {
            // Handle errors if vcpkg setup fails
            panic!("Failed to configure libpq via vcpkg.");
        }
    }
}
