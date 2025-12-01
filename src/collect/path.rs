//! Helpers for resolving paths in Verus modules.

use crate::defs::Path;
use std::collections::BTreeMap;
use syn::{
    ItemMod, ItemUse, UseTree,
    visit_mut::{VisitMut, visit_item_mod_mut},
};

/// Module stack.
#[derive(Debug)]
pub struct ModuleStack(Vec<String>);

impl ModuleStack {
    /// Create a new module stack.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Push a module onto the stack.
    pub fn push(&mut self, module: &str) {
        self.0.push(module.to_string());
    }

    /// Pop a module from the stack.
    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// Get the current module path.
    pub fn current(&self) -> Path {
        Path(self.0.clone())
    }

    /// Get the parent module path.
    pub fn parent(&self) -> Path {
        if self.0.is_empty() {
            Path(vec![])
        } else {
            let mut parent_segments = self.0.clone();
            parent_segments.pop();
            Path(parent_segments)
        }
    }

    /// Concatenate a symbol to the current module path.
    pub fn concat(&self, symbol: &str) -> Path {
        let mut path = self.0.clone();
        path.push(symbol.to_string());
        Path(path)
    }
}

/// Path resolver that gets a fully qualified path for a symbol.
#[derive(Debug)]
pub struct PathResolver {
    /// Module stack.
    module: ModuleStack,
    /// Mappings from symbol to fully qualified path.
    mappings: BTreeMap<String, Path>,
    /// Stack of resolver states for nested scopes.
    stack: Vec<BTreeMap<String, Path>>,
}

impl PathResolver {
    /// Create an empty path resolver.
    pub fn new() -> Self {
        Self {
            module: ModuleStack::new(),
            mappings: BTreeMap::new(),
            stack: Vec::new(),
        }
    }

    /// Resolve all paths in the given syntax tree.
    pub fn resolve_paths(&mut self, syntax: &mut syn::File) {
        self.visit_file_mut(syntax);
    }

    /// Resolve a given path to a fully qualified path.
    fn resolve_path(&self, path: &Path) -> Path {
        // Separate the first segment and the rest.
        let first_seg = path.0.first().unwrap();
        let rest: Vec<String> = path.0.iter().skip(1).cloned().collect();

        // Determine the prefix based on the first segment.
        let mut prefix = match first_seg.as_str() {
            "crate" => Path(vec!["crate".to_string()]),
            "self" => self.module.current(),
            "super" => self.module.parent(),
            _ => match self.mappings.get(first_seg) {
                Some(p) => p.clone(),
                None => Path(vec![first_seg.clone()]),
            },
        };

        // Append the rest of the segments.
        if rest.is_empty() {
            prefix
        } else {
            prefix.0.extend(rest.into_iter());
            prefix
        }
    }

    /// Enter a new module scope.
    fn enter_module(&mut self, module: &ItemMod) {
        self.stack.push(self.mappings.clone());
        self.module.push(&module.ident.to_string());
        // New module cannot use its parent's use statements.
        self.clear_mappings();
    }

    /// Exit the current module scope.
    fn exit_module(&mut self) {
        self.module.pop();
        // Restore previous mappings.
        self.mappings = self.stack.pop().unwrap();
    }

    /// Add all mappings from a use tree into the resolver.
    fn parse_use_tree(&mut self, use_tree: &UseTree, prefix: Path) {
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

impl VisitMut for PathResolver {
    fn visit_item_mod_mut(&mut self, i: &mut ItemMod) {
        self.enter_module(i);
        visit_item_mod_mut(self, i);
        self.exit_module();
    }

    fn visit_item_use_mut(&mut self, i: &mut ItemUse) {
        self.parse_use_tree(&i.tree, Path::empty());
    }

    fn visit_path_mut(&mut self, path: &mut syn::Path) {
        let mut resolved_path: syn::Path = self.resolve_path(&Path::from(path.clone())).into();
        for i in 0..resolved_path.segments.len() {
            if i >= resolved_path.segments.len() - path.segments.len() {
                resolved_path.segments[i] =
                    path.segments[i + path.segments.len() - resolved_path.segments.len()].clone();
            }
        }
        *path = resolved_path;
    }
}
