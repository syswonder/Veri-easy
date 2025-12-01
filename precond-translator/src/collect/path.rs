//! Helpers for resolving paths in Verus modules.

use crate::ast::Path;
use std::collections::BTreeMap;
use verus_syn::{ItemMod, UseTree};

/// Path resolver that gets a fully qualified path for a symbol.
#[derive(Debug, Clone)]
pub struct PathResolver {
    /// Current module
    module: Vec<String>,
    /// Mappings from symbol to fully qualified path.
    mappings: BTreeMap<String, Path>,
    /// Stack of resolver states for nested scopes.
    stack: Vec<BTreeMap<String, Path>>,
}

impl PathResolver {
    /// Create an empty path resolver.
    pub fn new() -> Self {
        Self {
            module: Vec::new(),
            mappings: BTreeMap::new(),
            stack: Vec::new(),
        }
    }

    /// Resolve a given path to a fully qualified path.
    pub fn resolve_path(&self, path: &Path) -> Path {
        let first_seg = path.0.first().unwrap();
        // Check if the first segment has a mapping.
        let mut prefix = match self.mappings.get(first_seg) {
            Some(p) => p.clone(),
            None => self.concat_module(&first_seg),
        };
        let segments: Vec<&String> = path.0.iter().skip(1).collect();
        if segments.is_empty() {
            prefix
        } else {
            prefix.0.extend(segments.into_iter().cloned());
            prefix
        }
    }

    /// Concatenate current module with the given name.
    pub fn concat_module(&self, name: &str) -> Path {
        if self.module.is_empty() {
            Path::from_string(&name)
        } else {
            Path(self.module.clone()).join(name.to_string())
        }
    }

    /// Enter a new module scope.
    pub fn enter_module(&mut self, module: &ItemMod) {
        self.stack.push(self.mappings.clone());
        self.module.push(module.ident.to_string());
        // New module cannot use its parent's use statements.
        self.clear_mappings();
    }

    /// Exit the current module scope.
    pub fn exit_module(&mut self) {
        self.module.pop();
        // Restore previous mappings.
        self.mappings = self.stack.pop().unwrap();
    }

    /// Add all mappings from a use tree into the resolver.
    pub fn parse_use_tree(&mut self, use_tree: &UseTree, prefix: Path) {
        match use_tree {
            UseTree::Path(use_path) => {
                self.parse_use_tree(&*use_path.tree, prefix.join(use_path.ident.to_string()));
            }
            UseTree::Name(use_name) => {
                self.mappings.insert(
                    use_name.ident.to_string(),
                    prefix.join(use_name.ident.to_string()),
                );
            }
            UseTree::Rename(use_rename) => {
                self.mappings.insert(
                    use_rename.rename.to_string(),
                    prefix.join(use_rename.ident.to_string()),
                );
            }
            UseTree::Glob(_) => {
                // Ignore glob imports for now.
            }
            UseTree::Group(use_group) => {
                for tree in &use_group.items {
                    self.parse_use_tree(tree, prefix.clone());
                }
            }
        }
    }

    /// Clear all mappings.
    fn clear_mappings(&mut self) {
        self.mappings.clear();
    }
}
