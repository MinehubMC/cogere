fn main() {
    println!("cargo:rerun-if-changed=migrations");

    download_htmx();
}

use std::path::Path;

const HTMX_VERSION: &str = "2.0.8";

fn download_htmx() {
    let dest = format!("assets/htmx-{HTMX_VERSION}.min.js");

    println!("cargo:rerun-if-changed={dest}");
    println!("cargo:rerun-if-changed=build.rs");

    if Path::new(&dest).exists() {
        return;
    }

    let url = format!("https://unpkg.com/htmx.org@{HTMX_VERSION}/dist/htmx.min.js");
    println!("cargo:warning=Downloading htmx {HTMX_VERSION}...");

    let body = ureq::get(&url)
        .call()
        .expect("Failed to download htmx")
        .body_mut()
        .read_to_string()
        .expect("Failed to read htmx response body");

    std::fs::write(&dest, body).expect("Failed to write htmx to assets/");

    println!("cargo:warning=htmx {HTMX_VERSION} saved to {dest}");
}
