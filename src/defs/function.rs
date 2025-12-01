use super::path::Path;
use super::types::Type;
use std::fmt::Debug;

/// Wrap `syn::Signature`.
#[derive(Clone)]
pub struct Signature(pub syn::Signature);

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.0.ident == other.0.ident
            && self.0.inputs.len() == other.0.inputs.len()
            && self
                .0
                .inputs
                .iter()
                .zip(other.0.inputs.iter())
                .all(|(a, b)| match (a, b) {
                    (syn::FnArg::Receiver(_), syn::FnArg::Receiver(_)) => true,
                    (syn::FnArg::Typed(a), syn::FnArg::Typed(b)) => type_eq(&a.ty, &b.ty),
                    _ => false,
                })
            && match (&self.0.output, &other.0.output) {
                (syn::ReturnType::Default, syn::ReturnType::Default) => true,
                (syn::ReturnType::Type(_, a), syn::ReturnType::Type(_, b)) => type_eq(a, b),
                _ => false,
            }
    }
}

/// Function metadata, including name, signature, impl type and trait (if any).
#[derive(Clone)]
pub struct FunctionMetadata {
    /// Fully-qualified name, e.g. "foo" or "MyType::bar" or "module::MyType::bar"
    pub name: Path,
    /// Function signature.
    pub signature: Signature,
    /// If the function is an impl method, the impl type.
    pub impl_type: Option<Type>,
}

impl FunctionMetadata {
    /// Create a new FunctionMetadata.
    pub fn new(name: Path, signature: Signature, impl_type: Option<Type>) -> Self {
        Self {
            name,
            signature,
            impl_type,
        }
    }

    /// Get the function identifier.
    pub fn ident(&self) -> String {
        self.signature.0.ident.to_string()
    }

    /// If the function is a constructor.
    pub fn is_constructor(&self) -> bool {
        self.impl_type.is_some() && self.signature.0.ident == "verieasy_new"
    }

    /// If the function is a getter.
    pub fn is_getter(&self) -> bool {
        self.impl_type.is_some()
            && matches!(
                self.signature.0.inputs.first(),
                Some(syn::FnArg::Receiver(_))
            )
            && self.signature.0.ident == "verieasy_get"
    }
}

impl Debug for FunctionMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.name)
    }
}

/// Function metadata and body.
#[derive(Clone)]
pub struct Function {
    /// Metadata of the function.
    pub metadata: FunctionMetadata,
    /// Function body.
    pub body: String,
}

impl Function {
    /// Create a new Function.
    pub fn new(metadata: FunctionMetadata, body: String) -> Self {
        Self { metadata, body }
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.metadata.name)
    }
}

/// Function shared by 2 source files, with same metadata but different bodies.
#[derive(Clone)]
pub struct CommonFunction {
    /// Metadata of the function.
    pub metadata: FunctionMetadata,
    /// Body from first source file.
    pub body1: String,
    /// Body from second source file.
    pub body2: String,
}

impl CommonFunction {
    /// Create a new CommonFunction.
    pub fn new(metadata: FunctionMetadata, body1: String, body2: String) -> Self {
        Self {
            metadata,
            body1,
            body2,
        }
    }
    /// Get the implementation type unchecked.
    pub fn impl_type(&self) -> &Type {
        self.metadata.impl_type.as_ref().unwrap()
    }
}

impl Debug for CommonFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.metadata.name)
    }
}

/// Precondition for a function.
#[derive(Clone)]
pub struct Precondition {
    /// Name of the **original** function (The check function name is derived from this).
    pub name: Path,
    /// Implementation type (if any).
    pub impl_type: Option<Type>,
}

impl Precondition {
    /// Construct from the Path of the original function.
    pub fn new(name: Path, is_method: bool) -> Self {
        let impl_type = if is_method {
            if name.0.len() >= 2 {
                Some(Type::from_path(name.parent().unwrap()))
            } else {
                None
            }
        } else {
            None
        };
        Self { name, impl_type }
    }

    /// Get the function identifier.
    pub fn ident(&self) -> String {
        self.name.0.last().cloned().unwrap()
    }

    /// The name of the check function.
    pub fn checker_name(&self) -> Path {
        if self.impl_type.is_some() {
            Path(vec![format!("verieasy_pre_{}", self.ident())])
        } else {
            let mut checker_name = self.name.clone();
            *checker_name.0.last_mut().unwrap() = format!("verieasy_pre_{}", self.ident());
            checker_name
        }
    }
}

impl Debug for Precondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Precondition {:?}", self.name)
    }
}

/// Convert a type to a string
fn type_to_string(ty: &syn::Type, sep: &str) -> String {
    match ty {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join(sep),
        _ => "unsupported".to_owned(),
    }
}

/// Check if two types are equal
fn type_eq(a: &syn::Type, b: &syn::Type) -> bool {
    type_to_string(a, "::") == type_to_string(b, "::")
}
