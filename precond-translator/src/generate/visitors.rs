//! Helper visitors for code generation.

use crate::ast::*;
use crate::visit::{self, Visit, VisitMut};
use proc_macro2::TokenStream;
use quote::quote;
use std::str::FromStr;

/// Visitor that generates code from an ast tree.
pub struct AstToCode {
    stack: Vec<TokenStream>,
}

impl AstToCode {
    /// Create a new generator.
    pub fn new() -> Self {
        AstToCode { stack: Vec::new() }
    }
    /// Get the generated code.
    pub fn get_code(&mut self) -> TokenStream {
        if self.stack.is_empty() {
            quote! { true }
        } else {
            self.stack.pop().unwrap()
        }
    }
}

impl Visit for AstToCode {
    fn visit_block(&mut self, block: &Block) {
        visit::visit_block(self, block);
        let mut exprs = Vec::new();
        for _ in 0..block.items.len() {
            exprs.push(self.stack.pop().unwrap());
        }
        exprs.reverse();
        let expr = quote! {
            { #(#exprs);* }
        };
        self.stack.push(expr);
    }

    fn visit_expr_lit(&mut self, lit: &ExprLit) {
        let expr = match lit {
            ExprLit::Bool(b) => {
                if *b {
                    quote! { true }
                } else {
                    quote! { false }
                }
            }
            ExprLit::Int(i) => {
                let int_value = *i;
                let ts = TokenStream::from_str(&int_value.to_string()).unwrap();
                quote! { #ts }
            }
            ExprLit::Str(s) => {
                let str_value = s;
                quote! { #str_value }
            }
        };
        self.stack.push(expr);
    }

    fn visit_expr_path(&mut self, path: &ExprPath) {
        let ts = TokenStream::from_str(&path.path.to_string()).unwrap();
        let expr = quote! {
            #ts
        };
        self.stack.push(expr);
    }

    fn visit_expr_index(&mut self, index: &ExprIndex) {
        visit::visit_expr_index(self, index);
        let idx = self.stack.pop().unwrap();
        let base = self.stack.pop().unwrap();
        let expr = quote! {
            (#base[#idx])
        };
        self.stack.push(expr);
    }

    fn visit_expr_cast(&mut self, cast: &ExprCast) {
        visit::visit_expr_cast(self, cast);
        let expr = self.stack.pop().unwrap();
        let to_type = TokenStream::from_str(&cast.to_type).unwrap();
        let expr = quote! {
            (#expr as #to_type)
        };
        self.stack.push(expr);
    }

    fn visit_expr_field(&mut self, field: &ExprField) {
        visit::visit_expr_field(self, field);
        let base = self.stack.pop().unwrap();
        let field_name = TokenStream::from_str(&field.field).unwrap();
        let expr = quote! {
            (#base.#field_name)
        };
        self.stack.push(expr);
    }

    fn visit_expr_binary(&mut self, binary: &ExprBinary) {
        visit::visit_expr_binary(self, binary);
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        let expr = if let BinaryOp::Imply = binary.op {
            quote! {
                (!#left || #right)
            }
        } else {
            let op = match binary.op {
                BinaryOp::Eq => quote! { == },
                BinaryOp::Ne => quote! { != },
                BinaryOp::Lt => quote! { < },
                BinaryOp::Le => quote! { <= },
                BinaryOp::Gt => quote! { > },
                BinaryOp::Ge => quote! { >= },
                BinaryOp::And => quote! { && },
                BinaryOp::Or => quote! { || },
                BinaryOp::Add => quote! { + },
                BinaryOp::Sub => quote! { - },
                BinaryOp::Mul => quote! { * },
                BinaryOp::Div => quote! { / },
                BinaryOp::Mod => quote! { % },
                _ => unreachable!(),
            };
            quote! {
                (#left #op #right)
            }
        };
        self.stack.push(expr);
    }

    fn visit_expr_unary(&mut self, unary: &ExprUnary) {
        visit::visit_expr_unary(self, unary);
        let expr = self.stack.pop().unwrap();
        let expr = match unary.op {
            UnaryOp::Not => quote! { (!#expr) },
        };
        self.stack.push(expr);
    }

    fn visit_expr_call(&mut self, call: &ExprCall) {
        visit::visit_expr_call(self, call);
        let mut args = Vec::new();
        for _ in 0..call.args.len() {
            args.push(self.stack.pop().unwrap());
        }
        args.reverse();
        let func = self.stack.pop().unwrap();
        let expr = quote! {
            #func(#(#args),*)
        };
        self.stack.push(expr);
    }

    fn visit_expr_method_call(&mut self, method_call: &ExprMethodCall) {
        visit::visit_expr_method_call(self, method_call);
        let mut args = Vec::new();
        for _ in 0..method_call.args.len() {
            args.push(self.stack.pop().unwrap());
        }
        args.reverse();
        let receiver = self.stack.pop().unwrap();

        let method = TokenStream::from_str(&method_call.method).unwrap();
        let expr = quote! {
            #receiver.#method(#(#args),*)
        };
        self.stack.push(expr);
    }
}

/// Visitor that removes "old" function calls by replacing them with their single argument.
pub struct RemoveOld;

impl VisitMut for RemoveOld {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Call(call) = expr {
            if call.func.path.to_string() == "old" && call.args.len() == 1 {
                let arg = call.args.pop().unwrap();
                *expr = arg;
                return;
            }
        }
        visit::visit_expr_mut(self, expr);
    }
}

/// Visitor that checks if all function calls are in the allowed list.
pub struct CheckFnCall<'a> {
    /// List of allowed function paths.
    fn_list: &'a [Path],
    /// Self type, for checking method calls.
    self_ty: Option<&'a Type>,
    /// Whether an invalid function call was found.
    pub aborted: bool,
}

impl<'a> CheckFnCall<'a> {
    pub fn new(fn_list: &'a [Path], self_ty: Option<&'a Type>) -> Self {
        CheckFnCall {
            fn_list,
            self_ty,
            aborted: false,
        }
    }
}

impl<'a> Visit for CheckFnCall<'a> {
    fn visit_expr_call(&mut self, call: &ExprCall) {
        if call.func.path.0.last().unwrap().starts_with("spec_") {
            // We assume function with "spec_" prefix always have an exec version.
            visit::visit_expr_call(self, call);
            return;
        }

        let func_path = if call.func.path.0.first().unwrap() == "Self" {
            if let Some(self_ty) = self.self_ty {
                // Convert "Self" to the actual type.
                let mut func_path = self_ty.as_path();
                func_path.0.extend(call.func.path.0.iter().cloned().skip(1));
                func_path
            } else {
                // No self type info, abort.
                self.aborted = true;
                return;
            }
        } else {
            call.func.path.clone()
        };

        if !self
            .fn_list
            .iter()
            .any(|p| p.to_string() == func_path.to_string())
        {
            self.aborted = true;
            return;
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, method_call: &ExprMethodCall) {
        if method_call.method.starts_with("spec_") {
            // We assume method with "spec_" prefix always have an exec version.
            visit::visit_expr_method_call(self, method_call);
            return;
        }

        if let Some(self_ty) = self.self_ty {
            // Convert method call to fully qualified path.
            let mut func_path = self_ty.as_path();
            func_path.0.push(method_call.method.clone());

            if !self
                .fn_list
                .iter()
                .any(|p| p.to_string() == func_path.to_string())
            {
                self.aborted = true;
                return;
            }
            visit::visit_expr_method_call(self, method_call);
        } else {
            // No self type info, abort.
            self.aborted = true;
        }
    }
}

/// Replace all function calls of "spec_foo" with "foo".
pub struct RemoveSpecPrefix;

impl VisitMut for RemoveSpecPrefix {
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        if let Some(last_seg) = call.func.path.0.last() {
            if last_seg.starts_with("spec_") {
                let new_name = last_seg.trim_start_matches("spec_").to_string();
                call.func.path.0.pop();
                call.func.path.0.push(new_name);
            }
        }
        visit::visit_expr_call_mut(self, call);
    }

    fn visit_expr_method_call_mut(&mut self, method_call: &mut ExprMethodCall) {
        if method_call.method.starts_with("spec_") {
            let new_name = method_call.method.trim_start_matches("spec_").to_string();
            method_call.method = new_name;
        }
        visit::visit_expr_method_call_mut(self, method_call);
    }
}
