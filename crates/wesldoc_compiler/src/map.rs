use wesl::syntax;
use wesldoc_ast::*;

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

impl Map<AddressSpace> for (syntax::AddressSpace, Option<syntax::AccessMode>) {
    fn map(&self) -> AddressSpace {
        let (address_space, access_mode) = self;
        match address_space {
            syntax::AddressSpace::Function => AddressSpace::Function,
            syntax::AddressSpace::Private => AddressSpace::Private,
            syntax::AddressSpace::Workgroup => AddressSpace::WorkGroup,
            syntax::AddressSpace::Uniform => AddressSpace::Uniform,
            syntax::AddressSpace::Storage => match access_mode {
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
                    load: true,
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

impl Map<BuiltinValue> for syntax::BuiltinValue {
    fn map(&self) -> BuiltinValue {
        match self {
            syntax::BuiltinValue::VertexIndex => BuiltinValue::VertexIndex,
            syntax::BuiltinValue::InstanceIndex => BuiltinValue::InstanceIndex,
            syntax::BuiltinValue::ClipDistances => BuiltinValue::ClipDistances,
            syntax::BuiltinValue::Position => BuiltinValue::Position,
            syntax::BuiltinValue::FrontFacing => BuiltinValue::FrontFacing,
            syntax::BuiltinValue::FragDepth => BuiltinValue::FragDepth,
            syntax::BuiltinValue::SampleIndex => BuiltinValue::SampleIndex,
            syntax::BuiltinValue::SampleMask => BuiltinValue::SampleMask,
            syntax::BuiltinValue::LocalInvocationId => BuiltinValue::LocalInvocationId,
            syntax::BuiltinValue::LocalInvocationIndex => BuiltinValue::LocalInvocationIndex,
            syntax::BuiltinValue::GlobalInvocationId => BuiltinValue::GlobalInvocationId,
            syntax::BuiltinValue::WorkgroupId => BuiltinValue::WorkgroupId,
            syntax::BuiltinValue::NumWorkgroups => BuiltinValue::NumWorkgroups,
            syntax::BuiltinValue::SubgroupInvocationId => BuiltinValue::SubgroupInvocationId,
            syntax::BuiltinValue::SubgroupSize => BuiltinValue::SubgroupSize,
        }
    }
}

impl Map<DiagnosticSeverity> for syntax::DiagnosticSeverity {
    fn map(&self) -> DiagnosticSeverity {
        match self {
            syntax::DiagnosticSeverity::Error => DiagnosticSeverity::Error,
            syntax::DiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
            syntax::DiagnosticSeverity::Info => DiagnosticSeverity::Info,
            syntax::DiagnosticSeverity::Off => DiagnosticSeverity::Off,
        }
    }
}

impl Map<InterpolationType> for syntax::InterpolationType {
    fn map(&self) -> InterpolationType {
        match self {
            syntax::InterpolationType::Perspective => InterpolationType::Perspective,
            syntax::InterpolationType::Linear => InterpolationType::Linear,
            syntax::InterpolationType::Flat => InterpolationType::Flat,
        }
    }
}

impl Map<InterpolationSampling> for syntax::InterpolationSampling {
    fn map(&self) -> InterpolationSampling {
        match self {
            syntax::InterpolationSampling::Center => InterpolationSampling::Center,
            syntax::InterpolationSampling::Centroid => InterpolationSampling::Centroid,
            syntax::InterpolationSampling::Sample => InterpolationSampling::Sample,
            syntax::InterpolationSampling::First => InterpolationSampling::First,
            syntax::InterpolationSampling::Either => InterpolationSampling::Either,
        }
    }
}
