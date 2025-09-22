use std::{fmt, str::FromStr};

pub use indexmap::{IndexMap, IndexSet};
pub use pulldown_cmark as md;
pub use semver::Version;

#[derive(Debug, Clone)]
pub struct WeslDocs {
    pub version: Version,
    pub root: Module,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub source: Option<String>,
    pub comment: Option<DocComment>,
    pub modules: Vec<Module>,
    pub constants: IndexMap<Ident, Item<Constant>>,
    pub overrides: IndexMap<Ident, Item<Override>>,
    pub global_variables: IndexMap<Ident, Item<GlobalVariable>>,
    pub structs: IndexMap<Ident, Item<Struct>>,
    pub functions: IndexMap<Ident, Item<Function>>,
    pub type_aliases: IndexMap<Ident, Item<TypeAlias>>,
    pub translate_time_features: IndexSet<String>,
}

impl Module {
    pub fn empty(name: String) -> Self {
        Self {
            name,
            source: None,
            comment: None,
            modules: Vec::new(),
            constants: IndexMap::new(),
            overrides: IndexMap::new(),
            global_variables: IndexMap::new(),
            structs: IndexMap::new(),
            functions: IndexMap::new(),
            type_aliases: IndexMap::new(),
            translate_time_features: IndexSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Conditional {
    False,
    True,
    Feature(Ident),
    Not(Box<Conditional>),
    And(Box<Conditional>, Box<Conditional>),
    Or(Box<Conditional>, Box<Conditional>),
}

impl fmt::Display for Conditional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Conditional::False => write!(f, "false"),
            Conditional::True => write!(f, "true"),
            Conditional::Feature(ident) => write!(f, "{}", ident),
            Conditional::Not(operand) => {
                if matches!(**operand, Conditional::And(_, _) | Conditional::Or(_, _)) {
                    write!(f, "!({})", operand)
                } else {
                    write!(f, "!{}", operand)
                }
            }
            Conditional::And(left, right) => {
                if matches!(**left, Conditional::Or(_, _)) {
                    write!(f, "({})", left)?;
                } else {
                    write!(f, "{}", left)?;
                }
                write!(f, " && ")?;
                if matches!(**right, Conditional::Or(_, _)) {
                    write!(f, "({})", right)?;
                } else {
                    write!(f, "{}", right)?;
                }
                Ok(())
            }
            Conditional::Or(left, right) => {
                if matches!(**left, Conditional::And(_, _)) {
                    write!(f, "({})", left)?;
                } else {
                    write!(f, "{}", left)?;
                }
                write!(f, " || ")?;
                if matches!(**right, Conditional::And(_, _)) {
                    write!(f, "({})", right)?;
                } else {
                    write!(f, "{}", right)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Module,
    Constant,
    Override,
    GlobalVariable,
    Struct,
    Function,
    TypeAlias,
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone)]
pub struct Item<T> {
    pub instances: Vec<T>,
    // Represents the combined conditional of all instances: a || b || c || ...
    // This is `None` if the combination is always true (tautology).
    pub conditional: Option<Conditional>,
}

impl<T> Default for Item<T> {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            conditional: None,
        }
    }
}

pub trait ItemInstance {
    const ITEM_KIND: ItemKind;
    fn conditional(&self) -> Option<&Conditional>;
    fn comment(&self) -> Option<&DocComment>;
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute>;
}

#[derive(Debug, Clone)]
pub struct Constant {
    pub name: Ident,
    pub ty: Option<TypeExpression>,
    pub init: Expression,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
    pub span: Option<Span>,
}

impl ItemInstance for Constant {
    const ITEM_KIND: ItemKind = ItemKind::Constant;
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
    fn comment(&self) -> Option<&DocComment> {
        self.comment.as_ref()
    }
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
}

#[derive(Debug, Clone)]
pub struct Override {
    pub name: Ident,
    pub ty: Option<TypeExpression>,
    pub init: Option<Expression>,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
    pub span: Option<Span>,
}

impl ItemInstance for Override {
    const ITEM_KIND: ItemKind = ItemKind::Override;
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
    fn comment(&self) -> Option<&DocComment> {
        self.comment.as_ref()
    }
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalVariable {
    pub name: Ident,
    pub space: AddressSpace,
    pub ty: Option<TypeExpression>,
    pub init: Option<Expression>,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
    pub span: Option<Span>,
}

impl ItemInstance for GlobalVariable {
    const ITEM_KIND: ItemKind = ItemKind::GlobalVariable;
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
    fn comment(&self) -> Option<&DocComment> {
        self.comment.as_ref()
    }
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AddressSpace {
    Function,
    Private,
    WorkGroup,
    Uniform,
    Storage { load: bool, store: bool },
    Handle,
    PushConstant,
}

impl fmt::Display for AddressSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressSpace::Function => write!(f, "<function>"),
            AddressSpace::Private => write!(f, "<private>"),
            AddressSpace::WorkGroup => write!(f, "<workgroup>"),
            AddressSpace::Uniform => write!(f, "<uniform>"),
            AddressSpace::Storage { store, .. } => {
                if *store {
                    write!(f, "<storage, read_write>")
                } else {
                    write!(f, "<storage>")
                }
            }
            AddressSpace::Handle => write!(f, ""),
            AddressSpace::PushConstant => write!(f, "<push_constant>"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Parenthesized(Box<Expression>),
    TypeOrIdentifier(TypeExpression),
    NotExpanded(Option<Span>),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Bool(bool),
    AbstractInt(i64),
    AbstractFloat(f64),
    I32(i32),
    U32(u32),
    F32(f32),
    F16(f32),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Bool(value) => write!(f, "{}", value),
            Literal::AbstractInt(value) => write!(f, "{}", value),
            Literal::AbstractFloat(value) => write!(f, "{}", value),
            Literal::I32(value) => write!(f, "{}", value),
            Literal::U32(value) => write!(f, "{}", value),
            Literal::F32(value) => write!(f, "{}", value),
            Literal::F16(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: Ident,
    pub members: Vec<StructMember>,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
    pub span: Option<Span>,
}

impl ItemInstance for Struct {
    const ITEM_KIND: ItemKind = ItemKind::Struct;
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
    fn comment(&self) -> Option<&DocComment> {
        self.comment.as_ref()
    }
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes
            .iter()
            .chain(self.members.iter().flat_map(|m| m.attributes.iter()))
    }
}

#[derive(Debug, Clone)]
pub struct StructMember {
    pub name: Ident,
    pub ty: TypeExpression,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
}

#[derive(Debug, Clone)]
pub enum TypeExpression {
    TypeIdentifier {
        name: Ident,
        template_args: Option<Vec<Expression>>,
    },
    Referenced {
        name: Ident,
        kind: ItemKind,
        def_path: DefinitionPath,
    },
}

#[derive(Debug, Clone)]
pub enum DefinitionPath {
    Absolute(Vec<String>),
    Package(String, Version, Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Ident,
    pub parameters: Vec<FunctionParameter>,
    pub ret: Option<TypeExpression>,
    pub attributes: Vec<Attribute>,
    pub return_attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
    pub span: Option<Span>,
}

impl ItemInstance for Function {
    const ITEM_KIND: ItemKind = ItemKind::Function;
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
    fn comment(&self) -> Option<&DocComment> {
        self.comment.as_ref()
    }
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes
            .iter()
            .chain(self.parameters.iter().flat_map(|p| p.attributes.iter()))
            .chain(self.return_attributes.iter())
    }
}

#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: Ident,
    pub ty: TypeExpression,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: Ident,
    pub ty: TypeExpression,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub comment: Option<DocComment>,
    pub span: Option<Span>,
}

impl ItemInstance for TypeAlias {
    const ITEM_KIND: ItemKind = ItemKind::TypeAlias;
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
    fn comment(&self) -> Option<&DocComment> {
        self.comment.as_ref()
    }
    fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(pub String);

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct DocComment {
    /// This is not escaped, e.g. it can contain `<script>` tags.
    pub unsafe_full: Vec<md::Event<'static>>,
    /// This is not escaped, e.g. it can contain `<script>` tags.
    pub unsafe_short: Vec<md::Event<'static>>,
    /// This is not escaped, e.g. it can contain `<script>` tags.
    pub unsafe_short_no_links: Vec<md::Event<'static>>,
}

#[derive(Debug, Clone)]
pub enum Attribute {
    Align(Expression),
    Binding(Expression),
    BlendSrc(Expression),
    Builtin(BuiltinValue),
    Const,
    Diagnostic {
        severity: DiagnosticSeverity,
        rule: String,
    },
    Group(Expression),
    Id(Expression),
    Interpolate {
        ty: InterpolationType,
        sampling: Option<InterpolationSampling>,
    },
    Invariant,
    Location(Expression),
    MustUse,
    Size(Expression),
    WorkgroupSize {
        x: Expression,
        y: Option<Expression>,
        z: Option<Expression>,
    },
    Vertex,
    Fragment,
    Compute,
    Custom {
        name: String,
        arguments: Option<Vec<Expression>>,
    },
}

impl Attribute {
    pub fn name(&self) -> &str {
        match self {
            Attribute::Align(_) => "align",
            Attribute::Binding(_) => "binding",
            Attribute::BlendSrc(_) => "blend_src",
            Attribute::Builtin(_) => "builtin",
            Attribute::Const => "const",
            Attribute::Diagnostic { .. } => "diagnostic",
            Attribute::Group(_) => "group",
            Attribute::Id(_) => "id",
            Attribute::Interpolate { .. } => "interpolate",
            Attribute::Invariant => "invariant",
            Attribute::Location(_) => "location",
            Attribute::MustUse => "must_use",
            Attribute::Size(_) => "size",
            Attribute::WorkgroupSize { .. } => "workgroup_size",
            Attribute::Vertex => "vertex",
            Attribute::Fragment => "fragment",
            Attribute::Compute => "compute",
            Attribute::Custom { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinValue {
    VertexIndex,
    InstanceIndex,
    ClipDistances,
    Position,
    FrontFacing,
    FragDepth,
    SampleIndex,
    SampleMask,
    LocalInvocationId,
    LocalInvocationIndex,
    GlobalInvocationId,
    WorkgroupId,
    NumWorkgroups,
    SubgroupInvocationId,
    SubgroupSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationType {
    Perspective,
    Linear,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationSampling {
    Center,
    Centroid,
    Sample,
    First,
    Either,
}

#[derive(Debug, Clone)]
pub struct IntraDocLink {
    pub def_path: DefinitionPath,
    pub kind: ItemKind,
    pub name: Ident,
}

impl fmt::Display for IntraDocLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "INTRA_DOC_LINK:")?;
        match &self.def_path {
            DefinitionPath::Absolute(components) => {
                write!(f, "absolute {}", components.join("::"))?;
            }
            DefinitionPath::Package(name, version, components) => {
                write!(f, "package {} {} {}", name, version, components.join("::"))?;
            }
        }
        write!(f, " ")?;
        match self.kind {
            ItemKind::Module => write!(f, "module")?,
            ItemKind::Constant => write!(f, "constant")?,
            ItemKind::Override => write!(f, "override")?,
            ItemKind::GlobalVariable => write!(f, "global_variable")?,
            ItemKind::Struct => write!(f, "struct")?,
            ItemKind::Function => write!(f, "function")?,
            ItemKind::TypeAlias => write!(f, "type_alias")?,
        }
        write!(f, " {}", self.name)?;
        Ok(())
    }
}

impl FromStr for IntraDocLink {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("INTRA_DOC_LINK:") {
            return Err(());
        }
        let s = s.trim_start_matches("INTRA_DOC_LINK:");

        let mut parts = s.split(' ');

        let def_path = match parts.next().ok_or(())? {
            "absolute" => {
                let components = parts
                    .next()
                    .ok_or(())?
                    .split("::")
                    .map(|s| s.to_string())
                    .collect();
                DefinitionPath::Absolute(components)
            }
            "package" => {
                let name = parts.next().ok_or(())?.to_string();
                let version = parts.next().ok_or(())?.parse().map_err(|_| ())?;
                let components = parts
                    .next()
                    .ok_or(())?
                    .split("::")
                    .map(|s| s.to_string())
                    .collect();
                DefinitionPath::Package(name, version, components)
            }
            _ => return Err(()),
        };

        let kind = match parts.next().ok_or(())? {
            "module" => ItemKind::Module,
            "constant" => ItemKind::Constant,
            "override" => ItemKind::Override,
            "global_variable" => ItemKind::GlobalVariable,
            "struct" => ItemKind::Struct,
            "function" => ItemKind::Function,
            "type_alias" => ItemKind::TypeAlias,
            _ => return Err(()),
        };

        let name = Ident(parts.next().ok_or(())?.to_string());

        Ok(IntraDocLink {
            def_path,
            kind,
            name,
        })
    }
}
