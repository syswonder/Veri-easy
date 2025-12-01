//! Types used in the AST for precondition representation.

use super::path::Path;

/// A type either generic or precise.
#[derive(Debug, Clone)]
pub enum Type {
    /// A generic type parameter.
    Generic(GenericType),
    /// A precise type.
    Precise(PreciseType),
}

impl Type {
    /// Get the path representation of the type.
    pub fn as_path(&self) -> Path {
        match self {
            Type::Generic(generic) => generic.as_path(),
            Type::Precise(precise) => precise.0.clone(),
        }
    }
}

impl TryFrom<verus_syn::Type> for Type {
    type Error = ();
    fn try_from(value: verus_syn::Type) -> Result<Self, Self::Error> {
        match value {
            verus_syn::Type::Path(type_path) => {
                let last = type_path.path.segments.last().cloned().unwrap();
                let path = Path::from(type_path.path);
                match last.arguments {
                    verus_syn::PathArguments::AngleBracketed(args) => {
                        // Collect generic arguments
                        let mut generics = Vec::new();
                        for arg in args.args {
                            match arg {
                                verus_syn::GenericArgument::Type(ty) => {
                                    let ty_converted = Type::try_from(ty).map_err(|_| ())?;
                                    generics.push(ty_converted);
                                }
                                _ => return Err(()),
                            }
                        }
                        Ok(Type::Generic(GenericType { path, generics }))
                    }
                    _ => Ok(Type::Precise(PreciseType(path))),
                }
            }
            _ => Err(()),
        }
    }
}

/// A precise type.
#[derive(Debug, Clone)]
pub struct PreciseType(pub Path);

/// A generic type parameter.
#[derive(Debug, Clone)]
pub struct GenericType {
    /// Name of the generic type parameter.
    pub path: Path,
    /// Instantiated types for this generic type parameter.
    pub generics: Vec<Type>,
}

impl GenericType {
    /// Get the path representation of the generic type.
    pub fn as_path(&self) -> Path {
        let mut full_path = self.path.clone();
        if !self.generics.is_empty() {
            let generics_str = self
                .generics
                .iter()
                .map(|ty| ty.as_path().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            full_path
                .0
                .last_mut()
                .unwrap()
                .push_str(&format!("<{}>", generics_str));
        }
        full_path
    }
}
