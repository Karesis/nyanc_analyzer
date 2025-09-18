use ast::Path; 
use nyanc_core::{FileId, Symbol};
use std::sync::Arc;

/// 这个 Trait 定义了所有分析（解析、类型检查等）过程
/// 所需要向“数据库”（即 CompilationContext）查询的所有能力。
pub trait AnalyzerDb {
    fn ast(&self, file_id: FileId) -> Arc<ast::Module>;
    fn resolve_module(&self, anchor_file: FileId, path: &Path) -> Option<FileId>;
    /// 将一个字符串切片转换为一个唯一的 Symbol
    fn intern_string(&self, s: &str) -> Symbol;
    // fn def_map(&self) -> Arc<DefMap>;
}