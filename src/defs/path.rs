use proc_macro2::TokenStream;
use std::str::FromStr;

/// Fully qualified path of a symbol, e.g., `std::vec::Vec`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path(pub Vec<String>);

impl Path {
    /// Create an empty path.
    pub fn empty() -> Self {
        Path(vec![])
    }

    /// Convert to string representation with "::" separator.
    pub fn to_string(&self) -> String {
        self.0.join("::")
    }

    /// Convert to a flattened identifier with "___" separator.
    pub fn to_ident(&self) -> String {
        self.0.join("___")
    }

    /// Get the last segment of the path.
    pub fn last(&self) -> Option<&String> {
        self.0.last()
    }

    /// Get a new path by removing the last segment.
    pub fn parent(&self) -> Option<Path> {
        if self.0.is_empty() {
            None
        } else {
            let mut parent_segments = self.0.clone();
            parent_segments.pop();
            Some(Path(parent_segments))
        }
    }

    /// Parse from a string representation with "::" separator.
    pub fn from_str(s: &str) -> Self {
        let segments: Vec<String> = s.split("::").map(|seg| seg.to_string()).collect();
        Path(segments)
    }

    /// Concatenate a string to this one.
    pub fn join(mut self, seg: String) -> Path {
        self.0.push(seg);
        self
    }
}

impl std::fmt::Debug for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<syn::Path> for Path {
    fn from(path: syn::Path) -> Self {
        let segments: Vec<String> = path
            .segments
            .into_iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        Path(segments)
    }
}

impl From<Path> for syn::Path {
    fn from(path: Path) -> Self {
        let segments: Vec<syn::PathSegment> = path
            .0
            .into_iter()
            .map(|seg| syn::PathSegment {
                ident: syn::Ident::new(&seg, proc_macro2::Span::call_site()),
                arguments: syn::PathArguments::None,
            })
            .collect();
        syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::from_iter(segments),
        }
    }
}

impl quote::ToTokens for Path {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ts = TokenStream::from_str(&self.to_string()).unwrap();
        tokens.extend(ts);
    }
}
