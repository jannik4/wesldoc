use wesl_docs::{Module, Version, WeslDocs};

pub trait WeslDocsExt {
    fn new(name: String, version: Version) -> Self;
}

impl WeslDocsExt for WeslDocs {
    fn new(name: String, version: Version) -> Self {
        Self {
            version,
            root: Module {
                name,
                source_url: None,
                modules: Vec::new(),
                constants: Vec::new(),
                global_variables: Vec::new(),
                structs: Vec::new(),
                functions: Vec::new(),
                shader_defs: Default::default(),
            },
            compiled_with: Default::default(),
        }
    }
}
