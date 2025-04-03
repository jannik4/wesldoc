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
        Ident::Named(self.name().clone())
    }
}
