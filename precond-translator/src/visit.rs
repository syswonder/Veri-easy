//! Expression tree traversal to walk a shared borrow of a expr tree.
#![allow(unused)]
use crate::ast::*;

/// Visitor trait for traversing the expression tree.
pub trait Visit {
    /// Visit a block.
    fn visit_block(&mut self, block: &Block) {
        visit_block(self, block);
    }
    /// Visit an expression.
    fn visit_expr(&mut self, expr: &Expr) {
        visit_expr(self, expr);
    }
    /// Visit a literal expression.
    fn visit_expr_lit(&mut self, lit: &ExprLit) {
        visit_expr_lit(self, lit);
    }
    /// Visit a path expression.
    fn visit_expr_path(&mut self, path: &ExprPath) {
        visit_expr_path(self, path);
    }
    /// Visit an index expression.
    fn visit_expr_index(&mut self, index: &ExprIndex) {
        visit_expr_index(self, index);
    }
    /// Visit a cast expression.
    fn visit_expr_cast(&mut self, cast: &ExprCast) {
        visit_expr_cast(self, cast);
    }
    /// Visit a field expression.
    fn visit_expr_field(&mut self, field: &ExprField) {
        visit_expr_field(self, field);
    }
    /// Visit a binary expression.
    fn visit_expr_binary(&mut self, binary: &ExprBinary) {
        visit_expr_binary(self, binary);
    }
    /// Visit a unary expression.
    fn visit_expr_unary(&mut self, unary: &ExprUnary) {
        visit_expr_unary(self, unary);
    }
    /// Visit a call expression.
    fn visit_expr_call(&mut self, call: &ExprCall) {
        visit_expr_call(self, call);
    }
    /// Visit a method call expression.
    fn visit_expr_method_call(&mut self, method_call: &ExprMethodCall) {
        visit_expr_method_call(self, method_call);
    }
}

/// Traverse a block with the given visitor.
pub fn visit_block<V: Visit + ?Sized>(visitor: &mut V, block: &Block) {
    for item in &block.items {
        match item {
            BlockItem::Expr(expr) => visitor.visit_expr(expr),
        }
    }
}

/// Traverse an expression tree with the given visitor.
pub fn visit_expr<V: Visit + ?Sized>(visitor: &mut V, expr: &Expr) {
    match expr {
        Expr::Lit(lit) => visitor.visit_expr_lit(lit),
        Expr::Path(path) => visitor.visit_expr_path(path),
        Expr::Index(index) => visitor.visit_expr_index(index),
        Expr::Cast(cast) => visitor.visit_expr_cast(cast),
        Expr::Field(field) => visitor.visit_expr_field(field),
        Expr::Binary(binary) => visitor.visit_expr_binary(binary),
        Expr::Unary(unary) => visitor.visit_expr_unary(unary),
        Expr::Call(call) => visitor.visit_expr_call(call),
        Expr::MethodCall(method_call) => visitor.visit_expr_method_call(method_call),
    }
}

/// Traverse a literal expression.
pub fn visit_expr_lit<V: Visit + ?Sized>(_visitor: &mut V, _lit: &ExprLit) {
    // No sub-expressions to visit.
}

/// Traverse a path expression.
pub fn visit_expr_path<V: Visit + ?Sized>(_visitor: &mut V, _path: &ExprPath) {
    // No sub-expressions to visit.
}

/// Traverse an index expression.
pub fn visit_expr_index<V: Visit + ?Sized>(visitor: &mut V, index: &ExprIndex) {
    visitor.visit_expr(&index.base);
    visitor.visit_expr(&index.index);
}

/// Traverse a cast expression.
pub fn visit_expr_cast<V: Visit + ?Sized>(visitor: &mut V, cast: &ExprCast) {
    visitor.visit_expr(&cast.expr);
}

/// Traverse a field expression.
pub fn visit_expr_field<V: Visit + ?Sized>(visitor: &mut V, field: &ExprField) {
    visitor.visit_expr(&field.base);
}

/// Traverse a binary expression.
pub fn visit_expr_binary<V: Visit + ?Sized>(visitor: &mut V, binary: &ExprBinary) {
    visitor.visit_expr(&binary.left);
    visitor.visit_expr(&binary.right);
}

/// Traverse a unary expression.
pub fn visit_expr_unary<V: Visit + ?Sized>(visitor: &mut V, unary: &ExprUnary) {
    visitor.visit_expr(&unary.expr);
}

/// Traverse a call expression.
pub fn visit_expr_call<V: Visit + ?Sized>(visitor: &mut V, call: &ExprCall) {
    visitor.visit_expr_path(&call.func);
    for arg in &call.args {
        visitor.visit_expr(arg);
    }
}

/// Traverse a method call expression.
pub fn visit_expr_method_call<V: Visit + ?Sized>(visitor: &mut V, method_call: &ExprMethodCall) {
    visitor.visit_expr(&method_call.receiver);
    for arg in &method_call.args {
        visitor.visit_expr(arg);
    }
}

/// Visitor trait for mutating an exclusive borrow of a expression tree in place.
pub trait VisitMut {
    /// Visit a block.
    fn visit_block_mut(&mut self, block: &mut Block) {
        visit_block_mut(self, block);
    }
    /// Visit an expression.
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        visit_expr_mut(self, expr);
    }
    /// Visit a literal expression.
    fn visit_expr_lit_mut(&mut self, lit: &mut ExprLit) {
        visit_expr_lit_mut(self, lit);
    }
    /// Visit a path expression.
    fn visit_expr_path_mut(&mut self, path: &mut ExprPath) {
        visit_expr_path_mut(self, path);
    }
    /// Visit an index expression.
    fn visit_expr_index_mut(&mut self, index: &mut ExprIndex) {
        visit_expr_index_mut(self, index);
    }
    /// Visit a cast expression.
    fn visit_expr_cast_mut(&mut self, cast: &mut ExprCast) {
        visit_expr_cast_mut(self, cast);
    }
    /// Visit a field expression.
    fn visit_expr_field_mut(&mut self, field: &mut ExprField) {
        visit_expr_field_mut(self, field);
    }
    /// Visit a binary expression.
    fn visit_expr_binary_mut(&mut self, binary: &mut ExprBinary) {
        visit_expr_binary_mut(self, binary);
    }
    /// Visit a unary expression.
    fn visit_expr_unary_mut(&mut self, unary: &mut ExprUnary) {
        visit_expr_unary_mut(self, unary);
    }
    /// Visit a call expression.
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        visit_expr_call_mut(self, call);
    }
    /// Visit a method call expression.
    fn visit_expr_method_call_mut(&mut self, method_call: &mut ExprMethodCall) {
        visit_expr_method_call_mut(self, method_call);
    }
}

/// Traverse a block with the given mutable visitor.
pub fn visit_block_mut<V: VisitMut + ?Sized>(visitor: &mut V, block: &mut Block) {
    for item in &mut block.items {
        match item {
            BlockItem::Expr(expr) => visitor.visit_expr_mut(expr),
        }
    }
}

/// Traverse an expression tree with the given mutable visitor.
pub fn visit_expr_mut<V: VisitMut + ?Sized>(visitor: &mut V, expr: &mut Expr) {
    match expr {
        Expr::Lit(lit) => visitor.visit_expr_lit_mut(lit),
        Expr::Path(path) => visitor.visit_expr_path_mut(path),
        Expr::Index(index) => visitor.visit_expr_index_mut(index),
        Expr::Cast(cast) => visitor.visit_expr_cast_mut(cast),
        Expr::Field(field) => visitor.visit_expr_field_mut(field),
        Expr::Binary(binary) => visitor.visit_expr_binary_mut(binary),
        Expr::Unary(unary) => visitor.visit_expr_unary_mut(unary),
        Expr::Call(call) => visitor.visit_expr_call_mut(call),
        Expr::MethodCall(method_call) => visitor.visit_expr_method_call_mut(method_call),
    }
}

/// Traverse a literal expression.
pub fn visit_expr_lit_mut<V: VisitMut + ?Sized>(_visitor: &mut V, _lit: &mut ExprLit) {
    // No sub-expressions to visit.
}

/// Traverse a path expression.
pub fn visit_expr_path_mut<V: VisitMut + ?Sized>(_visitor: &mut V, _path: &mut ExprPath) {
    // No sub-expressions to visit.
}

/// Traverse an index expression.
pub fn visit_expr_index_mut<V: VisitMut + ?Sized>(visitor: &mut V, index: &mut ExprIndex) {
    visitor.visit_expr_mut(&mut index.base);
    visitor.visit_expr_mut(&mut index.index);
}

/// Traverse a cast expression.
pub fn visit_expr_cast_mut<V: VisitMut + ?Sized>(visitor: &mut V, cast: &mut ExprCast) {
    visitor.visit_expr_mut(&mut cast.expr);
}

/// Traverse a field expression.
pub fn visit_expr_field_mut<V: VisitMut + ?Sized>(visitor: &mut V, field: &mut ExprField) {
    visitor.visit_expr_mut(&mut field.base);
}

/// Traverse a binary expression.
pub fn visit_expr_binary_mut<V: VisitMut + ?Sized>(visitor: &mut V, binary: &mut ExprBinary) {
    visitor.visit_expr_mut(&mut binary.left);
    visitor.visit_expr_mut(&mut binary.right);
}

/// Traverse a unary expression.
pub fn visit_expr_unary_mut<V: VisitMut + ?Sized>(visitor: &mut V, unary: &mut ExprUnary) {
    visitor.visit_expr_mut(&mut unary.expr);
}

/// Traverse a call expression.
pub fn visit_expr_call_mut<V: VisitMut + ?Sized>(visitor: &mut V, call: &mut ExprCall) {
    visitor.visit_expr_path_mut(&mut call.func);
    for arg in &mut call.args {
        visitor.visit_expr_mut(arg);
    }
}

/// Traverse a method call expression.
pub fn visit_expr_method_call_mut<V: VisitMut + ?Sized>(
    visitor: &mut V,
    method_call: &mut ExprMethodCall,
) {
    visitor.visit_expr_mut(&mut method_call.receiver);
    for arg in &mut method_call.args {
        visitor.visit_expr_mut(arg);
    }
}
