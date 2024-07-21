pub mod pdf_document;
pub mod pdf_extractor;
pub mod pdfv1;
use once_cell::sync::OnceCell;

pub use pdfium_render::prelude::*;

static PDFIUM: OnceCell<Pdfium> = OnceCell::new(); // static initializers must impl Sync + Send

#[cfg(target_vendor = "apple")]
pub fn init(_statically_linked: &str) -> Pdfium {
	extern crate link_cplusplus;

	let bindings = Pdfium::bind_to_statically_linked_library().unwrap();
	Pdfium::new(bindings)
}

#[cfg(not(target_vendor = "apple"))]
fn init_dynamically_linked(binary_folder: &str, binary_name: &str, lib_bytes: &[u8]) -> Pdfium {
	use std::{fs, path::Path};
	let binary_path = format!("{}/{}", binary_folder, binary_name);
	let binary_folder = Path::new(binary_folder);
	let binary_path = Path::new(&binary_path);
	if !binary_path.exists() {
		fs::create_dir_all(binary_folder).unwrap();
		fs::write(&binary_path, lib_bytes).unwrap_or_default();
	}
	let bindings =
		Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&binary_folder))
			.unwrap();

	Pdfium::new(bindings)
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn init(binary_folder: &String) -> (Pdfium, String) {
	let binary_name = "pdfium.dll";
	let lib_bytes = include_bytes!("pdfium-windows/x64/lib/pdfium.dll");
	(
		init_dynamically_linked(binary_folder, binary_name, lib_bytes),
		Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
	)
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub fn init(binary_folder: &String) -> (Pdfium, String) {
	let binary_name = "pdfium.dll";
	let lib_bytes = include_bytes!("pdfium-windows/arm64/lib/pdfium.dll.lib");
	(
		init_dynamically_linked(binary_folder, binary_name, lib_bytes),
		Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
	)
}

#[cfg(target_os = "linux")]
pub fn init(binary_folder: &String) -> (Pdfium, String) {
	use std::path::Path;

	let binary_name = "libpdfium.so";
	let lib_bytes = include_bytes!("pdfium-linux/lib/libpdfium.so");
	(
		init_dynamically_linked(binary_folder, binary_name, lib_bytes),
		Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
	)
}

#[cfg(target_os = "android")]
pub fn init(binary_folder: &String) -> (Pdfium, String) {
	let binary_name = "libpdfium.so";
	let lib_bytes = if cfg!(target_arch = "aarch64") {
		let lib_bytes = include_bytes!("pdfium-android/arm64-v8a/lib/libpdfium.so");
		(
			init_dynamically_linked(binary_folder, binary_name, lib_bytes),
			Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
		)
	} else if cfg!(target_arch = "armv7") {
		let lib_bytes = include_bytes!("pdfium-android/armeabi-v7a/lib/libpdfium.so");
		(
			init_dynamically_linked(binary_folder, binary_name, lib_bytes),
			Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
		)
	} else if cfg!(target_arch = "x86") {
		let lib_bytes = include_bytes!("pdfium-android/x86/lib/libpdfium.so");
		(
			init_dynamically_linked(binary_folder, binary_name, lib_bytes),
			Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
		)
	} else if cfg!(target_arch = "x86_64") {
		let lib_bytes = include_bytes!("pdfium-android/x86_64/lib/libpdfium.so");
		(
			init_dynamically_linked(binary_folder, binary_name, lib_bytes),
			Path::new(binary_folder).join("libpdfium.so").to_str().unwrap().to_string(),
		)
	} else {
		panic!("unsupported target architecture")
	};

	lib_bytes
}

pub fn init_static_pdf(binary_folder: &String) {
	let (pdfium, _) = init(binary_folder);
	PDFIUM.set(pdfium).ok().expect("Can't set PDFIUM");
}
