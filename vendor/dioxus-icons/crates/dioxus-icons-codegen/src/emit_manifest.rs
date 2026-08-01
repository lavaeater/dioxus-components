use anyhow::{Context, Result};

use crate::fetch::LUCIDE_VERSION;
use crate::parse::{Icon, manifest};

pub fn emit_manifest(icons: &[Icon]) -> Result<String> {
    let manifest = manifest(LUCIDE_VERSION, icons);
    let json = serde_json::to_string_pretty(&manifest).context("serializing manifest")?;
    Ok(json.replace('<', "\\u003c").replace('>', "\\u003e"))
}
