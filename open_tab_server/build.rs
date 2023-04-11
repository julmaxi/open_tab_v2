use std::process::Command;
use std::env;
use fs_extra::dir::{copy, CopyOptions};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src-react/ballot_submission_form");

    let app_name = "src-react/ballot_submission_form";
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = crate_dir.join("static/ballot_submission_form");

    // Change directory to the react app directory
    Command::new("sh")
        .arg("-c")
        .arg(format!("cd {} && npm install && npm run build", app_name))
        .output()
        .expect("Failed to execute command");

    // Create the destination directory if it doesn't exist
    std::fs::create_dir_all(&out_dir).expect("Failed to create destination directory");

    // Copy the build directory to the destination directory
    let options = CopyOptions {
        content_only: true,
        overwrite: true,
        ..Default::default()
    };
    copy(format!("{}/build/", app_name), &out_dir, &options).expect("Failed to copy build directory");
}