use crate::Result;
use include_dir::{Dir, include_dir};
use std::{borrow::Cow, collections::HashMap, fs, path::Path, sync::LazyLock};

struct StaticFile {
    hashed_file_name: String,
    hashed_path: String, // includes the hashed file name
    contents: &'static [u8],
    parent: &'static Dir<'static>,
}

static FILES: LazyLock<HashMap<String, StaticFile>> = LazyLock::new(|| {
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
                    let hashed_file_name = match file_name.split_once('.') {
                        Some((name, ext)) => format!("{}-{}.{}", name, short_hash, ext),
                        None => format!("{}-{}", file_name, short_hash),
                    };

                    let hashed_path = f.path().with_file_name(&hashed_file_name);

                    files.insert(
                        display_path(f.path()),
                        StaticFile {
                            hashed_file_name,
                            hashed_path: display_path(&hashed_path),
                            contents: f.contents(),
                            parent: dir,
                        },
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
    for file in FILES.values() {
        let target = static_path.join(&file.hashed_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // Replace any references to other static files in the contents
        // Fix/hack so that references in the font css files to woff2 files are replaced with the
        // hashed file names
        let mut contents = file.contents;
        let contents_str;
        if let Ok(contents_utf8) = str::from_utf8(contents) {
            let mut contents_utf8 = Cow::Borrowed(contents_utf8);
            let mut replaced = false;

            while let Some((left, right)) = contents_utf8.split_once("HASHED{")
                && let Some((to_replace, right)) = right.split_once('}')
            {
                let to_replace_path = display_path(&file.parent.path().join(to_replace));
                let to_replace_hashed = FILES
                    .get(&to_replace_path)
                    .map(|f| f.hashed_file_name.as_str())
                    .unwrap_or_else(|| {
                        wesldoc_report::warn!("could not find static file: {}", to_replace_path);
                        to_replace
                    });

                contents_utf8 = Cow::Owned(format!("{}{}{}", left, to_replace_hashed, right));
                replaced = true;
            }

            if replaced {
                contents_str = contents_utf8.to_string();
                contents = contents_str.as_bytes();
            }
        }

        fs::write(&target, contents)?;
    }

    Ok(())
}

pub fn resolve(path: &str) -> Option<&str> {
    FILES.get(path).map(|file| file.hashed_path.as_str())
}

fn display_path(path: &Path) -> String {
    let mut display_path = path.to_str().unwrap().to_string();
    if cfg!(windows) {
        display_path = display_path.replace("\\", "/");
    }
    display_path
}
