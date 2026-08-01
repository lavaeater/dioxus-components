use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::fetch::{LUCIDE_COMMIT, LUCIDE_VERSION, LUCIDE_ZIP_SHA256};

pub fn compute_inputs_hash() -> Result<String> {
    let codegen_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hasher = Sha256::new();
    hasher.update(b"lucide-version:");
    hasher.update(LUCIDE_VERSION.as_bytes());
    hasher.update(b"\nlucide-commit:");
    hasher.update(LUCIDE_COMMIT.as_bytes());
    hasher.update(b"\nlucide-sha256:");
    hasher.update(LUCIDE_ZIP_SHA256.as_bytes());
    hasher.update(b"\n");
    hash_rust_sources(&codegen_src, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_rust_sources(dir: &Path, hasher: &mut Sha256) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"));
    entries.sort();

    for path in entries {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("invalid file name {}", path.display()))?;
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        hasher.update(name.as_bytes());
        hasher.update(b"\n");
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
        hasher.update(b"\n");
    }
    Ok(())
}
