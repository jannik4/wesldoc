use crate::Result;
use include_dir::{Dir, include_dir};
use std::{collections::HashMap, fs, path::Path, sync::LazyLock};

static FILES: LazyLock<HashMap<String, (String, &'static [u8])>> = LazyLock::new(|| {
    static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

    let mut files = HashMap::new();

    let mut dirs = vec![&STATIC_DIR];
    while let Some(dir) = dirs.pop() {
        for entry in dir.entries() {
            match entry {
                include_dir::DirEntry::Dir(d) => dirs.push(d),
                include_dir::DirEntry::File(f) => {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(f.contents());
                    let hash = hasher.finalize();
                    let short_hash = &hash.to_hex()[..12];

                    let file_name = f.path().file_name().unwrap().to_str().unwrap();
                    let new_file_name = match file_name.split_once('.') {
                        Some((name, ext)) => format!("{}-{}.{}", name, short_hash, ext),
                        None => format!("{}-{}", file_name, short_hash),
                    };

                    let hashed_path = f.path().with_file_name(new_file_name);

                    files.insert(
                        display_path(f.path()),
                        (display_path(&hashed_path), f.contents()),
                    );
                }
            }
        }
    }

    files
});

pub fn write_static_files(base_path: &Path) -> Result<()> {
    let static_path = base_path.join("-/static");

    // Prepare directories
    fs::remove_dir_all(&static_path).ok();
    fs::create_dir_all(&static_path)?;

    // Write hashed static files
    for (hashed_path, contents) in FILES.values() {
        let target = static_path.join(hashed_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, contents)?;
    }

    Ok(())
}

pub fn resolve(path: &str) -> Option<&str> {
    FILES.get(path).map(|(hashed_path, _)| hashed_path.as_str())
}

fn display_path(path: &Path) -> String {
    let mut display_path = path.to_str().unwrap().to_string();
    if cfg!(windows) {
        display_path = display_path.replace("\\", "/");
    }
    display_path
}
