use camino::Utf8PathBuf;
use std::env;
use std::process::Command;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/helix.udl");
    
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("OUT_DIR: {}", out_dir);
    let udl_file = "src/helix.udl";
    
    uniffi::generate_scaffolding(udl_file)
        .expect("Failed to generate scaffolding");

    napi_build::setup();

    // build_dependencies().expect("Failed to build dependencies");
}

fn build_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let crates = vec![
        "helix-engine",
        "helix-gateway",
        "protocol",
    ];

    let target_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("native");
    std::fs::create_dir_all(&target_dir)?;

    for crate_name in crates {
        println!("Building {}", crate_name);
        
        let status = Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--manifest-path",
                &format!("../{}/Cargo.toml", crate_name)
            ])
            .status()?;

        if !status.success() {
            return Err(format!("Failed to build {}", crate_name).into());
        } else {
            println!("Successfully built {}", crate_name);
        }

        let lib_name = if cfg!(target_os = "windows") {
            format!("{}.dll", crate_name.replace("-", "_"))
        } else if cfg!(target_os = "macos") {
            format!("lib{}.dylib", crate_name.replace("-", "_"))
        } else {
            format!("lib{}.so", crate_name.replace("-", "_"))
        };

        let src_path = PathBuf::from("..")
            .join(crate_name)
            .join("target")
            .join("release")
            .join(&lib_name);

        let dst_path = target_dir.join(&lib_name);
        println!("Copying {} to {}", src_path.display(), dst_path.display());
        if !src_path.exists() {
            return Err(format!("Source path does not exist: {:?}", src_path).into());
        }

        if !dst_path.parent().unwrap().exists() {
            return Err(format!("Destination directory does not exist: {}", dst_path.parent().unwrap().display()).into());
        }

        std::fs::copy(src_path, dst_path).unwrap();
    }

    Ok(())
}