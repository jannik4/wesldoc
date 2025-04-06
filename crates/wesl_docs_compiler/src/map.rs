use wesl::syntax;
use wesl_docs::*;

pub trait Map<T>: Sized {
    fn map(&self) -> T;
}

pub fn map<T: Map<U>, U>(value: &T) -> U {
    value.map()
}

impl Map<Ident> for syntax::Ident {
    fn map(&self) -> Ident {
        Ident(self.name().clone())
    }
}

impl Map<AddressSpace> for syntax::AddressSpace {
    fn map(&self) -> AddressSpace {
        match self {
            syntax::AddressSpace::Function => AddressSpace::Function,
            syntax::AddressSpace::Private => AddressSpace::Private,
            syntax::AddressSpace::Workgroup => AddressSpace::WorkGroup,
            syntax::AddressSpace::Uniform => AddressSpace::Uniform,
            syntax::AddressSpace::Storage(access_mode) => match access_mode {
                Some(syntax::AccessMode::Read) => AddressSpace::Storage {
                    load: true,
                    store: false,
                },
                Some(syntax::AccessMode::Write) => AddressSpace::Storage {
                    load: false,
                    store: true,
                },
                Some(syntax::AccessMode::ReadWrite) => AddressSpace::Storage {
                    load: true,
                    store: true,
                },
                None => AddressSpace::Storage {
                    load: false,
                    store: false,
                },
            },
            syntax::AddressSpace::Handle => AddressSpace::Handle,
        }
    }
}

impl Map<Literal> for syntax::LiteralExpression {
    fn map(&self) -> Literal {
        match *self {
            syntax::LiteralExpression::Bool(value) => Literal::Bool(value),
            syntax::LiteralExpression::AbstractInt(value) => Literal::AbstractInt(value),
            syntax::LiteralExpression::AbstractFloat(value) => Literal::AbstractFloat(value),
            syntax::LiteralExpression::I32(value) => Literal::I32(value),
            syntax::LiteralExpression::U32(value) => Literal::U32(value),
            syntax::LiteralExpression::F32(value) => Literal::F32(value),
            syntax::LiteralExpression::F16(value) => Literal::F16(value),
        }
    }
}
