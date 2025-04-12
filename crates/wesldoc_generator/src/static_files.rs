use crate::Result;
use include_dir::{Dir, include_dir};
use std::{fs, path::Path};

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

pub fn write_static_files(base_path: &Path) -> Result<()> {
    let static_path = base_path.join("-/static");

    // Prepare directories
    fs::remove_dir_all(&static_path).ok();
    fs::create_dir_all(&static_path)?;

    // Copy static files
    STATIC_DIR.extract(static_path)?;

    Ok(())
}
