use serde::Serialize;
use wesl_docs::{Module, WeslDocs};

pub fn all_items(doc: &WeslDocs) -> Vec<Item> {
    let mut items = Vec::new();
    all_items_module(&doc.root, &[], &mut items);
    items.sort();
    items
}

fn all_items_module(module: &Module, parent: &[String], items: &mut Vec<Item>) {
    let path = parent
        .iter()
        .cloned()
        .chain([module.name.clone()])
        .collect::<Vec<_>>();
    for inner in &module.modules {
        all_items_module(inner, &path, items);

        items.push(Item::new(
            path.clone(),
            inner.name.clone(),
            ItemKind::Module,
        ));
    }

    for name in module.constants.keys() {
        items.push(Item::new(path.clone(), name.0.clone(), ItemKind::Constant));
    }

    for name in module.global_variables.keys() {
        items.push(Item::new(
            path.clone(),
            name.0.clone(),
            ItemKind::GlobalVariable,
        ));
    }

    for name in module.structs.keys() {
        items.push(Item::new(path.clone(), name.0.clone(), ItemKind::Struct));
    }

    for name in module.functions.keys() {
        items.push(Item::new(path.clone(), name.0.clone(), ItemKind::Function));
    }

    for name in module.type_aliases.keys() {
        items.push(Item::new(path.clone(), name.0.clone(), ItemKind::TypeAlias));
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item {
    path: Vec<String>,
    name: String,
    kind: ItemKind,
    url: String,
}

impl Item {
    fn new(path: Vec<String>, name: String, kind: ItemKind) -> Self {
        let mut url = path.join("/");
        match kind {
            ItemKind::Module => url.push_str(&format!("/{}/index.html", name)),
            ItemKind::Constant => url.push_str(&format!("/const.{}.html", name)),
            ItemKind::GlobalVariable => url.push_str(&format!("/var.{}.html", name)),
            ItemKind::Struct => url.push_str(&format!("/struct.{}.html", name)),
            ItemKind::Function => url.push_str(&format!("/fn.{}.html", name)),
            ItemKind::TypeAlias => url.push_str(&format!("/type.{}.html", name)),
        }

        Self {
            path,
            name,
            kind,
            url,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum ItemKind {
    Module,
    Constant,
    GlobalVariable,
    Struct,
    Function,
    TypeAlias,
}
