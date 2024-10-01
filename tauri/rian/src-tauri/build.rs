#[cfg(target_os = "windows")]
extern crate vcpkg;

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=libpq");
        let pq_installed = setup_libpq_vcpkg();
        if !pq_installed {
            panic!("libpq not found");
        }
    }
    tauri_build::build()
}

#[cfg(target_env = "msvc")]
fn setup_libpq_vcpkg() -> bool {
    vcpkg::Config::new()
        .target_triplet("x64-windows-static-md")
        .find_package("libpq")
        .map(|_| {
            // found libpq, now try to find openssl
            if let Err(_) = vcpkg::Config::new().find_package("openssl") {
                eprintln!("Warning: OpenSSL not found, continuing without it.");
            }

            // Link additional Windows libraries
            println!("cargo:rustc-link-lib=crypt32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=secur32");
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=wldap32");
        })
        .is_ok()
}
