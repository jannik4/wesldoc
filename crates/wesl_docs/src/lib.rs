use std::fmt;

pub use indexmap::{IndexMap, IndexSet};
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
    pub modules: Vec<Module>,
    pub constants: IndexMap<Ident, Item<Constant>>,
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
            modules: Vec::new(),
            constants: IndexMap::new(),
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
    fn conditional(&self) -> Option<&Conditional>;
}

#[derive(Debug, Clone)]
pub struct Constant {
    pub name: Ident,
    pub ty: Option<TypeExpression>,
    pub init: Expression,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
    pub span: Option<Span>,
}

impl ItemInstance for Constant {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
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
    pub span: Option<Span>,
}

impl ItemInstance for GlobalVariable {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
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
    Unknown,
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
    pub span: Option<Span>,
}

impl ItemInstance for Struct {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct StructMember {
    pub name: Ident,
    pub ty: TypeExpression,
    pub attributes: Vec<Attribute>,
    pub conditional: Option<Conditional>,
}

#[derive(Debug, Clone)]
pub enum TypeExpression {
    TypeIdentifier {
        name: Ident,
        template_args: Option<Vec<Expression>>,
    },
    ReferencedType {
        name: Ident,
        kind: ItemKind,
        def_path: Option<DefinitionPath>,
    },
    Unknown,
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
    pub span: Option<Span>,
}

impl ItemInstance for Function {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
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
    pub span: Option<Span>,
}

impl ItemInstance for TypeAlias {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinValue {
    VertexIndex,
    InstanceIndex,
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
