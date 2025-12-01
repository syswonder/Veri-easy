//! Definition of AST operators.

/// Binary operators supported.
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Eq,
    Ne,
    Gt,
    Ge,
    And,
    Or,
    Imply,
}

// Convert Verus AST binary operator to our BinaryOp
impl TryFrom<verus_syn::BinOp> for BinaryOp {
    type Error = ();
    fn try_from(op: verus_syn::BinOp) -> Result<Self, Self::Error> {
        match op {
            verus_syn::BinOp::Add(_) => Ok(BinaryOp::Add),
            verus_syn::BinOp::Sub(_) => Ok(BinaryOp::Sub),
            verus_syn::BinOp::Mul(_) => Ok(BinaryOp::Mul),
            verus_syn::BinOp::Div(_) => Ok(BinaryOp::Div),
            verus_syn::BinOp::Rem(_) => Ok(BinaryOp::Mod),
            verus_syn::BinOp::Lt(_) => Ok(BinaryOp::Lt),
            verus_syn::BinOp::Le(_) => Ok(BinaryOp::Le),
            verus_syn::BinOp::Eq(_) => Ok(BinaryOp::Eq),
            verus_syn::BinOp::Ne(_) => Ok(BinaryOp::Ne),
            verus_syn::BinOp::Gt(_) => Ok(BinaryOp::Gt),
            verus_syn::BinOp::Ge(_) => Ok(BinaryOp::Ge),
            verus_syn::BinOp::And(_) => Ok(BinaryOp::And),
            verus_syn::BinOp::Or(_) => Ok(BinaryOp::Or),
            verus_syn::BinOp::Imply(_) => Ok(BinaryOp::Imply),
            _ => Err(()),
        }
    }
}

/// Unary operators supported.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Not,
}

// Convert Verus AST unary operator to our UnaryOp
impl TryFrom<verus_syn::UnOp> for UnaryOp {
    type Error = ();
    fn try_from(op: verus_syn::UnOp) -> Result<Self, Self::Error> {
        match op {
            verus_syn::UnOp::Not(_) => Ok(UnaryOp::Not),
            _ => Err(()),
        }
    }
}
