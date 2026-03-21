fn main() {
    println!("cargo:rerun-if-changed=migrations");

    download_htmx();
    download_bulma();
    embed_version();
}

use std::path::Path;

const HTMX_VERSION: &str = "2.0.8";
const BULMA_VERSION: &str = "1.0.4";

fn embed_version() {
    println!(
        "cargo:rustc-env=COGERE_VERSION={}",
        std::env::var("CARGO_PKG_VERSION").unwrap()
    );

    let sha = std::env::var("GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    println!("cargo:rustc-env=COGERE_GIT_SHA={sha}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}

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

fn download_bulma() {
    let dest = format!("assets/bulma-{BULMA_VERSION}.min.css");

    println!("cargo:rerun-if-changed={dest}");
    println!("cargo:rerun-if-changed=build.rs");

    if Path::new(&dest).exists() {
        return;
    }

    let url = format!("https://cdn.jsdelivr.net/npm/bulma@{BULMA_VERSION}/css/bulma.min.css");
    println!("cargo:warning=Downloading bulma {BULMA_VERSION}...");

    let body = ureq::get(&url)
        .call()
        .expect("Failed to download bulma")
        .body_mut()
        .read_to_string()
        .expect("Failed to read bulma response body");

    std::fs::write(&dest, body).expect("Failed to write bulma to assets/");

    println!("cargo:warning=bulma {BULMA_VERSION} saved to {dest}");
}
