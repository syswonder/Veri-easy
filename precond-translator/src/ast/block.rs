//! Definition of AST block and related types.

use super::expr::Expr;

/// Block type that contains a sequence of items.
#[derive(Debug, Clone)]
pub struct Block {
    /// Items in the block.
    pub items: Vec<BlockItem>,
}

/// An item in a block, currently only expressions are supported.
#[derive(Debug, Clone)]
pub enum BlockItem {
    Expr(Expr),
}

impl TryFrom<verus_syn::Block> for Block {
    type Error = ();
    fn try_from(block: verus_syn::Block) -> Result<Self, Self::Error> {
        let mut items = Vec::new();
        for stmt in block.stmts {
            match stmt {
                verus_syn::Stmt::Expr(expr, _) => {
                    let expr_converted = Expr::try_from(expr).map_err(|_| ())?;
                    items.push(BlockItem::Expr(expr_converted));
                }
                _ => return Err(()),
            }
        }
        Ok(Block { items })
    }
}
