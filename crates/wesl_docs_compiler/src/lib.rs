mod docs_ext;

use docs_ext::WeslDocsExt;
use std::collections::HashMap;
use wesl::CompileResult;
use wesl_docs::{Module, Version, WeslDocs};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct WeslPackage {
    pub version: Version,
    pub dependencies: HashMap<String, Version>,
    pub root: WeslModule,
}

pub struct WeslModule {
    pub name: String,
    pub compiled: Option<CompileResult>,
    pub submodules: Vec<WeslModule>,
}

pub fn compile(package: WeslPackage) -> Result<WeslDocs> {
    let docs = WeslDocs::new(package.root.name, package.version);

    // TODO: ...

    Ok(docs)
}
