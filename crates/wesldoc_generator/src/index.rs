use crate::Result;
use askama::Template;
use std::{
    fs::{self, File},
    path::Path,
};

pub fn update(base_path: &Path) -> Result<()> {
    let packages = get_packages(base_path)?;

    let template = IndexTemplate {
        packages: &packages,
    };
    template.write_into(&mut File::create(base_path.join("index.html"))?)?;

    Ok(())
}

fn get_packages(base_path: &Path) -> Result<Vec<String>> {
    let mut packages = Vec::new();

    for entry in fs::read_dir(base_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && fs::exists(path.join("common.js"))? {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                packages.push(name.to_string());
            }
        }
    }

    Ok(packages)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    packages: &'a [String],
}
