// analyzer/src/ty.rs

use hir::{HirId, Type};
use std::collections::HashMap;

/// TypeMap 是类型检查阶段的核心产物。
/// 它将每一个表达式（甚至未来每一个变量声明）的 HirId 映射到其推断出的具体类型。
#[derive(Debug, Default)]
pub struct TypeMap {
    /// 将表达式 ID 映射到其类型
    pub expr_types: HashMap<HirId, Type>,
    // 未来我们可能还会需要...
    // /// 将变量定义 ID 映射到其类型
    // pub var_types: HashMap<DefId, Type>,
}

impl TypeMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_expr_type(&mut self, expr_id: HirId, ty: Type) {
        self.expr_types.insert(expr_id, ty);
    }

    pub fn get_expr_type(&self, expr_id: HirId) -> Option<&Type> {
        self.expr_types.get(&expr_id)
    }
}