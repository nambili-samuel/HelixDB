use uniffi_bindgen::bindings::SwiftBindingGenerator;
use camino::Utf8PathBuf;
use std::path::PathBuf;

pub fn main() {
    let udl_file = "src/helix.udl";
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::create_dir_all(&out_dir).unwrap();
    let out_dir = Utf8PathBuf::from(out_dir);
    let bindings_dir = out_dir.join("bindings");
    std::fs::create_dir_all(&bindings_dir).unwrap();
    // Generate Swift bindings
    uniffi_bindgen::generate_bindings(
        &Utf8PathBuf::from(udl_file),
        None,
        SwiftBindingGenerator {},
        Some(&out_dir.join("swift")),
        None,
        None,
        false,
    ).unwrap();

    
}