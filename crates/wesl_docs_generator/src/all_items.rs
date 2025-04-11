use serde::Serialize;
use wesl_docs::{
    Attribute, DocComment, Ident, IndexMap, Item, ItemInstance, ItemKind, Module, WeslDocs,
};

pub fn all_items(doc: &WeslDocs) -> impl Serialize {
    let mut items = Vec::new();
    all_items_module(&doc.root, &[], &mut items);
    items.sort();
    items
}

fn all_items_module(
    module: &Module,
    parent: &[String],
    serialized_items: &mut Vec<SerializedItem>,
) {
    let path = parent
        .iter()
        .cloned()
        .chain([module.name.clone()])
        .collect::<Vec<_>>();
    for inner in &module.modules {
        all_items_module(inner, &path, serialized_items);

        serialized_items.push(SerializedItem::new(
            path.clone(),
            inner.name.clone(),
            [],
            inner.comment.as_ref(),
            SerializedItemKind::Module,
        ));
    }

    add_items(&module.constants, path.clone(), serialized_items);
    add_items(&module.overrides, path.clone(), serialized_items);
    add_items(&module.global_variables, path.clone(), serialized_items);
    add_items(&module.structs, path.clone(), serialized_items);
    add_items(&module.functions, path.clone(), serialized_items);
    add_items(&module.type_aliases, path.clone(), serialized_items);
}

fn add_items<T>(
    items: &IndexMap<Ident, Item<T>>,
    path: Vec<String>,
    serialized_items: &mut Vec<SerializedItem>,
) where
    T: ItemInstance,
{
    for (name, item) in items {
        serialized_items.push(SerializedItem::new(
            path.clone(),
            name.0.clone(),
            item.instances.iter().flat_map(|i| i.all_attributes()),
            item.instances[0].comment(),
            T::ITEM_KIND.into(),
        ));
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct SerializedItem {
    path: Vec<String>,
    name: String,
    attributes: Vec<String>,
    comment: String,
    kind: SerializedItemKind,
    url: String,
}

impl SerializedItem {
    fn new<'a>(
        path: Vec<String>,
        name: String,
        attributes: impl IntoIterator<Item = &'a Attribute>,
        comment: Option<&DocComment>,
        kind: SerializedItemKind,
    ) -> Self {
        let mut url = path.join("/");
        match kind {
            SerializedItemKind::Module => url.push_str(&format!("/{}/index.html", name)),
            SerializedItemKind::Constant => url.push_str(&format!("/const.{}.html", name)),
            SerializedItemKind::Override => url.push_str(&format!("/override.{}.html", name)),
            SerializedItemKind::GlobalVariable => url.push_str(&format!("/var.{}.html", name)),
            SerializedItemKind::Struct => url.push_str(&format!("/struct.{}.html", name)),
            SerializedItemKind::Function => url.push_str(&format!("/fn.{}.html", name)),
            SerializedItemKind::TypeAlias => url.push_str(&format!("/type.{}.html", name)),
        }

        Self {
            path,
            name,
            attributes: attributes
                .into_iter()
                .map(|attr| format!("@{}", attr.name()))
                .collect(),
            comment: crate::render_doc_comment_short_no_links(comment),
            kind,
            url,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum SerializedItemKind {
    Module,
    Constant,
    Override,
    GlobalVariable,
    Struct,
    Function,
    TypeAlias,
}

impl From<ItemKind> for SerializedItemKind {
    fn from(kind: ItemKind) -> Self {
        match kind {
            ItemKind::Module => SerializedItemKind::Module,
            ItemKind::Constant => SerializedItemKind::Constant,
            ItemKind::Override => SerializedItemKind::Override,
            ItemKind::GlobalVariable => SerializedItemKind::GlobalVariable,
            ItemKind::Struct => SerializedItemKind::Struct,
            ItemKind::Function => SerializedItemKind::Function,
            ItemKind::TypeAlias => SerializedItemKind::TypeAlias,
        }
    }
}
