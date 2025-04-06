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
