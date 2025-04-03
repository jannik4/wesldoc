use std::fmt;

pub use indexmap::{IndexMap, IndexSet};
pub use semver::Version;

#[derive(Debug, Clone)]
pub struct WeslDocs {
    pub version: Version,
    pub root: Module,
    pub compiled_with: IndexMap<String, ShaderDefValue>,
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

#[derive(Debug, Clone, Copy)]
pub enum ShaderDefValue {
    Bool(bool),
    Int(i32),
    UInt(u32),
}

impl fmt::Display for ShaderDefValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderDefValue::Bool(value) => write!(f, "{}", value),
            ShaderDefValue::Int(value) => write!(f, "{}i", value),
            ShaderDefValue::UInt(value) => write!(f, "{}u", value),
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
    pub ty: Type,
    pub init: Expression,
    pub conditional: Option<Conditional>,
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
    pub binding: Option<ResourceBinding>,
    pub ty: Type,
    pub init: Option<Expression>,
    pub conditional: Option<Conditional>,
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

#[derive(Debug, Clone, Copy)]
pub struct ResourceBinding {
    pub group: u32,
    pub binding: u32,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Unknown,
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(literal) => write!(f, "{}", literal),
            Expression::Unknown => write!(f, ".."),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    F64(f64),
    F32(f32),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    Bool(bool),

    AbstractInt(i64),
    AbstractFloat(f64),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::F64(value) => write!(f, "{}", value),
            Literal::F32(value) => write!(f, "{}", value),
            Literal::U32(value) => write!(f, "{}", value),
            Literal::I32(value) => write!(f, "{}", value),
            Literal::U64(value) => write!(f, "{}", value),
            Literal::I64(value) => write!(f, "{}", value),
            Literal::Bool(value) => write!(f, "{}", value),

            Literal::AbstractInt(value) => write!(f, "{}", value),
            Literal::AbstractFloat(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: Ident,
    pub members: Vec<StructMember>,
    pub conditional: Option<Conditional>,
}

impl ItemInstance for Struct {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct StructMember {
    pub name: Ident,
    pub ty: Type,
    pub binding: Option<Binding>,
    pub conditional: Option<Conditional>,
}

#[derive(Debug, Clone)]
pub enum Type {
    Named {
        name: String,
        def_path: Option<DefinitionPath>,
    },
    Pointer(Box<Type>),
    PointerWithAddressSpace {
        base: Box<Type>,
        address_space: &'static str,
        maybe_access: Option<&'static str>,
    },
    ArrayConstant(Box<Type>, Option<u32>),
    ArrayDynamic(Box<Type>),
    BindingArrayConstant(Box<Type>, Option<u32>),
    BindingArrayDynamic(Box<Type>),
    Unnamed,
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
    pub ret: Option<Type>,
    pub conditional: Option<Conditional>,
}

impl ItemInstance for Function {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: Ident,
    pub ty: Type,
    pub binding: Option<Binding>,
    pub conditional: Option<Conditional>,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: Ident,
    pub ty: Type,
    pub conditional: Option<Conditional>,
}

impl ItemInstance for TypeAlias {
    fn conditional(&self) -> Option<&Conditional> {
        self.conditional.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident {
    Named(String),
    Unnamed,
}

impl<T> From<Option<T>> for Ident
where
    T: Into<String>,
{
    fn from(name: Option<T>) -> Self {
        match name {
            Some(name) => Ident::Named(name.into()),
            None => Ident::Unnamed,
        }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ident::Named(name) => write!(f, "{}", name),
            Ident::Unnamed => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Binding {
    BuiltIn(BuiltIn),
    Location {
        location: u32,
        second_blend_source: bool,
        interpolation: Option<Interpolation>,
        sampling: Option<Sampling>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BuiltIn {
    Position { invariant: bool },
    ViewIndex,
    BaseInstance,
    BaseVertex,
    ClipDistance,
    CullDistance,
    InstanceIndex,
    PointSize,
    VertexIndex,
    FragDepth,
    PointCoord,
    FrontFacing,
    PrimitiveIndex,
    SampleIndex,
    SampleMask,
    GlobalInvocationId,
    LocalInvocationId,
    LocalInvocationIndex,
    WorkGroupId,
    WorkGroupSize,
    NumWorkGroups,
    NumSubgroups,
    SubgroupId,
    SubgroupSize,
    SubgroupInvocationId,
    DrawID,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Interpolation {
    Perspective,
    Linear,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sampling {
    Center,
    Centroid,
    Sample,
    First,
    Either,
}
