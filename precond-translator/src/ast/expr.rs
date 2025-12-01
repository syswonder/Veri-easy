//! Definition of AST expression and related types.

use super::{
    op::{BinaryOp, UnaryOp},
    path::Path,
};

/// A type that expresses "checkable" expressions derived from Verus spec AST.
///
/// This Expr type is intentionally simple, expressive enough for boolean logic, field accesses,
/// indexing, type casts, and calls to spec functions/methods.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal value.
    Lit(ExprLit),
    /// Field or path access: a::f
    Path(ExprPath),
    /// Indexing: base[index]
    Index(ExprIndex),
    /// Type cast: a as T
    Cast(ExprCast),
    /// Field access: base.field
    Field(ExprField),
    /// Binary operations: + - * / % < <= == != > >= && || ...
    Binary(ExprBinary),
    /// Unary operations: !
    Unary(ExprUnary),
    /// Call to a spec function.
    Call(ExprCall),
    /// Call to a spec method.
    MethodCall(ExprMethodCall),
}

// Convert Verus AST expression to our RequireExpr
impl TryFrom<verus_syn::Expr> for Expr {
    type Error = ();
    fn try_from(expr: verus_syn::Expr) -> Result<Self, Self::Error> {
        match expr {
            verus_syn::Expr::Lit(lit) => {
                let literal = ExprLit::try_from(lit.lit).map_err(|_| ())?;
                Ok(Expr::Lit(literal))
            }
            verus_syn::Expr::Cast(cast_expr) => {
                let expt = ExprCast::try_from(cast_expr).map_err(|_| ())?;
                Ok(Expr::Cast(expt))
            }
            verus_syn::Expr::Index(index_expr) => {
                let index = ExprIndex::try_from(index_expr).map_err(|_| ())?;
                Ok(Expr::Index(index))
            }
            verus_syn::Expr::Path(p) => {
                let path = ExprPath::try_from(p).map_err(|_| ())?;
                Ok(Expr::Path(path))
            }
            verus_syn::Expr::Field(field_expr) => {
                let field = ExprField::try_from(field_expr).map_err(|_| ())?;
                Ok(Expr::Field(field))
            }
            verus_syn::Expr::Binary(bin_expr) => {
                let bin = ExprBinary::try_from(bin_expr).map_err(|_| ())?;
                Ok(Expr::Binary(bin))
            }
            verus_syn::Expr::Unary(un_expr) => {
                let un = ExprUnary::try_from(un_expr).map_err(|_| ())?;
                Ok(Expr::Unary(un))
            }
            verus_syn::Expr::Call(call_expr) => {
                let call = ExprCall::try_from(call_expr).map_err(|_| ())?;
                Ok(Expr::Call(call))
            }
            verus_syn::Expr::MethodCall(method_call) => {
                let method = ExprMethodCall::try_from(method_call).map_err(|_| ())?;
                Ok(Expr::MethodCall(method))
            }
            verus_syn::Expr::View(view) => {
                let method = ExprMethodCall::try_from(view).map_err(|_| ())?;
                Ok(Expr::MethodCall(method))
            }
            _ => Err(()),
        }
    }
}

/// Literal value.
#[derive(Debug, Clone)]
pub enum ExprLit {
    Bool(bool),
    Int(i128),
    Str(String),
}

// Convert Verus AST literal to our Literal
impl TryFrom<verus_syn::Lit> for ExprLit {
    type Error = ();
    fn try_from(lit: verus_syn::Lit) -> Result<Self, Self::Error> {
        match lit {
            verus_syn::Lit::Bool(b) => Ok(ExprLit::Bool(b.value)),
            verus_syn::Lit::Int(i) => {
                let int_value = i.base10_parse::<i128>().map_err(|_| ())?;
                Ok(ExprLit::Int(int_value))
            }
            verus_syn::Lit::Str(s) => Ok(ExprLit::Str(s.value())),
            _ => Err(()),
        }
    }
}

/// Path to a symbol, represented as a vector of segments.
#[derive(Debug, Clone)]
pub struct ExprPath {
    pub path: Path,
}

impl TryFrom<verus_syn::ExprPath> for ExprPath {
    type Error = ();
    fn try_from(path: verus_syn::ExprPath) -> Result<Self, Self::Error> {
        let path = Path::try_from(path.path).map_err(|_| ())?;
        Ok(ExprPath { path })
    }
}

/// Index expression: base[index].
#[derive(Debug, Clone)]
pub struct ExprIndex {
    pub base: Box<Expr>,
    pub index: Box<Expr>,
}

impl TryFrom<verus_syn::ExprIndex> for ExprIndex {
    type Error = ();
    fn try_from(index_expr: verus_syn::ExprIndex) -> Result<Self, Self::Error> {
        let base = Box::new(Expr::try_from(*index_expr.expr)?);
        let index = Box::new(Expr::try_from(*index_expr.index)?);
        Ok(ExprIndex { base, index })
    }
}

/// Type cast expression: expr as to_type.
#[derive(Debug, Clone)]
pub struct ExprCast {
    pub expr: Box<Expr>,
    pub to_type: String,
}

impl TryFrom<verus_syn::ExprCast> for ExprCast {
    type Error = ();
    fn try_from(cast_expr: verus_syn::ExprCast) -> Result<Self, Self::Error> {
        let expr = Box::new(Expr::try_from(*cast_expr.expr)?);
        let to_type = match *cast_expr.ty {
            verus_syn::Type::Path(type_path) => type_path
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            _ => return Err(()),
        };
        Ok(ExprCast { expr, to_type })
    }
}

/// Field access: base.field
#[derive(Debug, Clone)]
pub struct ExprField {
    pub base: Box<Expr>,
    pub field: String,
}

impl TryFrom<verus_syn::ExprField> for ExprField {
    type Error = ();
    fn try_from(field_expr: verus_syn::ExprField) -> Result<Self, Self::Error> {
        let base = Box::new(Expr::try_from(*field_expr.base)?);
        let field = match field_expr.member {
            verus_syn::Member::Named(ident) => ident.to_string(),
            verus_syn::Member::Unnamed(index) => index.index.to_string(),
        };
        Ok(ExprField { base, field })
    }
}

/// Binary expression: left op right.
#[derive(Debug, Clone)]
pub struct ExprBinary {
    pub op: BinaryOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

impl TryFrom<verus_syn::ExprBinary> for ExprBinary {
    type Error = ();
    fn try_from(bin_expr: verus_syn::ExprBinary) -> Result<Self, Self::Error> {
        let left = Box::new(Expr::try_from(*bin_expr.left)?);
        let right = Box::new(Expr::try_from(*bin_expr.right)?);
        let op = BinaryOp::try_from(bin_expr.op).map_err(|_| ())?;
        Ok(ExprBinary { op, left, right })
    }
}

/// Unary expression: op expr.
#[derive(Debug, Clone)]
pub struct ExprUnary {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

impl TryFrom<verus_syn::ExprUnary> for ExprUnary {
    type Error = ();
    fn try_from(un_expr: verus_syn::ExprUnary) -> Result<Self, Self::Error> {
        let expr = Box::new(Expr::try_from(*un_expr.expr)?);
        let op = UnaryOp::try_from(un_expr.op).map_err(|_| ())?;
        Ok(ExprUnary { op, expr })
    }
}

/// Function call expression: name(args).
#[derive(Debug, Clone)]
pub struct ExprCall {
    pub func: ExprPath,
    pub args: Vec<Expr>,
}

impl TryFrom<verus_syn::ExprCall> for ExprCall {
    type Error = ();
    fn try_from(call_expr: verus_syn::ExprCall) -> Result<Self, Self::Error> {
        let func = match *call_expr.func {
            verus_syn::Expr::Path(p) => p.try_into().map_err(|_| ())?,
            _ => return Err(()),
        };
        let args = call_expr
            .args
            .into_iter()
            .map(|arg| Expr::try_from(arg))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ExprCall { func, args })
    }
}

/// Method call expression: receiver.method(args).
#[derive(Debug, Clone)]
pub struct ExprMethodCall {
    pub receiver: Box<Expr>,
    pub method: String,
    pub args: Vec<Expr>,
}

impl TryFrom<verus_syn::ExprMethodCall> for ExprMethodCall {
    type Error = ();
    fn try_from(method_call: verus_syn::ExprMethodCall) -> Result<Self, Self::Error> {
        let receiver = Box::new(Expr::try_from(*method_call.receiver)?);
        let method = method_call.method.to_string();
        let args = method_call
            .args
            .into_iter()
            .map(|arg| Expr::try_from(arg))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ExprMethodCall {
            receiver,
            method,
            args,
        })
    }
}

impl TryFrom<verus_syn::View> for ExprMethodCall {
    type Error = ();
    fn try_from(view: verus_syn::View) -> Result<Self, Self::Error> {
        let receiver = Box::new(Expr::try_from(*view.expr)?);
        Ok(ExprMethodCall {
            receiver,
            method: "view".to_string(),
            args: vec![],
        })
    }
}
