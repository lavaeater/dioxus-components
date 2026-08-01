use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const LUCIDE_VERSION: &str = "1.14.0";
pub const LUCIDE_COMMIT: &str = "56e49f12166312f04af4cfd862621c93cf583979";
pub const LUCIDE_ZIP_SHA256: &str =
    "6f2238cc72de14f807f8d4cb5acbb232da1d2d1e3916dee95f8adc9c83c82394";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub fn ensure_lucide(workspace_root: &Path) -> Result<PathBuf> {
    let vendor_dir = workspace_root
        .join("vendor")
        .join(format!("lucide-{LUCIDE_VERSION}"));
    let icons_dir = vendor_dir.join("icons");

    if is_complete_cache(&icons_dir)? {
        return Ok(icons_dir);
    }

    fs::create_dir_all(workspace_root.join("vendor")).context("creating vendor directory")?;

    let tmp_dir = workspace_root
        .join("vendor")
        .join(format!(".lucide-{LUCIDE_VERSION}.tmp"));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).context("removing stale Lucide temp directory")?;
    }
    fs::create_dir_all(&tmp_dir).context("creating Lucide temp directory")?;

    let bytes = download_lucide_zip()?;
    validate_checksum(&bytes)?;
    extract_icons_zip(&bytes, &tmp_dir)?;

    if vendor_dir.exists() {
        fs::remove_dir_all(&vendor_dir).context("removing incomplete Lucide cache")?;
    }
    fs::rename(&tmp_dir, &vendor_dir).context("moving Lucide cache into place")?;

    if !is_complete_cache(&icons_dir)? {
        bail!("Lucide cache did not contain matching icons/*.svg and icons/*.json files");
    }

    Ok(icons_dir)
}

pub fn check_lucide_updates() -> Result<()> {
    let release = latest_lucide_release()?;
    let latest_version = normalize_version(&release.tag_name);
    let update_available = latest_version != LUCIDE_VERSION;

    println!("pinned_version={LUCIDE_VERSION}");
    println!("latest_version={latest_version}");
    println!("latest_url={}", release.html_url);
    println!("update_available={update_available}");

    if update_available {
        println!();
        println!(
            "Lucide {latest_version} is available. The pinned dioxus-icons version is {LUCIDE_VERSION}."
        );
        println!(
            "Upgrade by updating LUCIDE_VERSION, LUCIDE_COMMIT, LUCIDE_ZIP_SHA256, regenerating icons, and reviewing the generated diff."
        );
    }

    Ok(())
}

fn is_complete_cache(icons_dir: &Path) -> Result<bool> {
    if !icons_dir.is_dir() {
        return Ok(false);
    }

    let mut svg_count = 0usize;
    let mut json_count = 0usize;
    for entry in fs::read_dir(icons_dir).context("reading Lucide icons cache")? {
        let entry = entry?;
        match entry.path().extension().and_then(|ext| ext.to_str()) {
            Some("svg") => svg_count += 1,
            Some("json") => json_count += 1,
            _ => {}
        }
    }

    Ok(svg_count > 0 && svg_count == json_count)
}

fn download_lucide_zip() -> Result<Vec<u8>> {
    let url = format!(
        "https://github.com/lucide-icons/lucide/releases/download/{version}/lucide-icons-{version}.zip",
        version = LUCIDE_VERSION
    );

    let mut response = ureq::get(&url)
        .call()
        .with_context(|| format!("fetching Lucide release asset from {url}"))?;

    response
        .body_mut()
        .read_to_vec()
        .context("reading Lucide release asset")
}

fn latest_lucide_release() -> Result<GitHubRelease> {
    let url = "https://api.github.com/repos/lucide-icons/lucide/releases/latest";
    let mut response = ureq::get(url)
        .header("User-Agent", "dioxus-icons-codegen")
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("fetching latest Lucide release metadata")?;
    let bytes = response
        .body_mut()
        .read_to_vec()
        .context("reading latest Lucide release metadata")?;
    let text = String::from_utf8(bytes).context("decoding latest Lucide release metadata")?;
    serde_json::from_str(&text).context("parsing latest Lucide release metadata")
}

fn normalize_version(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

fn validate_checksum(bytes: &[u8]) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != LUCIDE_ZIP_SHA256 {
        bail!(
            "Lucide {LUCIDE_VERSION} checksum mismatch: expected {LUCIDE_ZIP_SHA256}, got {actual}"
        );
    }
    Ok(())
}

fn extract_icons_zip(bytes: &[u8], destination: &Path) -> Result<()> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("opening Lucide icons zip")?;

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .with_context(|| format!("reading zip entry {index}"))?;

        if file.is_dir() {
            continue;
        }

        let Some(enclosed_name) = file.enclosed_name() else {
            bail!("zip entry has unsafe path: {}", file.name());
        };

        if !enclosed_name.starts_with("icons/") {
            continue;
        }

        let out_path = destination.join(enclosed_name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).context("creating Lucide cache subdirectory")?;
        }

        let mut output = fs::File::create(&out_path)
            .with_context(|| format!("creating {}", out_path.display()))?;
        std::io::copy(&mut file, &mut output)
            .with_context(|| format!("extracting {}", out_path.display()))?;
    }

    Ok(())
}
