// analyzer/src/resolver.rs

use crate::db::AnalyzerDb;
use ast::Item as AstItem; // 使用 `as` 来避免与 hir::Item 的命名冲突
use nyanc_core::{Symbol, FileId};
use hir::DefId;
use std::collections::HashMap;
use std::sync::Arc;
use std::collections::HashSet;
use std::collections::VecDeque;

// --- 数据结构定义 ---

/// 存储一个顶层项目种类的“标签”
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Function,
    Struct,
}

/// 存储一个顶层项目的精简信息。
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub def_id: DefId,
    pub name: Symbol,
    pub kind: ItemKind, // 现在这个类型被定义了
    pub ast_node: Arc<ast::Item>,
}

/// “定义地图”，整个项目中所有顶层项目（函数、结构体等）的中央登记处。
#[derive(Debug, Default)]
pub struct DefMap {
    pub items: HashMap<DefId, ItemDef>,
}

impl DefMap {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 一个简单的 DefId 分配器
#[derive(Debug, Default)]
pub struct DefIdAllocator {
    counter: u32,
}

impl DefIdAllocator {
    pub fn new() -> Self { Self::default() }
    pub fn new_def_id(&mut self) -> DefId {
        let id = self.counter;
        self.counter += 1;
        DefId(id)
    }
}

/// Resolver 是我们的“图书管理员”，负责扫描代码并建立 DefMap。
pub struct Resolver<'db, DB: ?Sized + AnalyzerDb> {
    db: &'db DB,
    id_allocator: DefIdAllocator,
    def_map: DefMap,
}

impl<'db, DB: ?Sized + AnalyzerDb> Resolver<'db, DB> {
    pub fn new(db: &'db DB) -> Self {
        Self {
            db,
            id_allocator: DefIdAllocator::new(),
            def_map: DefMap::new(),
        }
    }

    /// 这是“定义收集”的入口点。
    /// 它将从一个入口文件开始，递归地遍历整个 crate，并返回完整的 DefMap。
    pub fn collect_defs_crate(mut self, entry_file: FileId) -> DefMap {
        let mut worklist: VecDeque<FileId> = VecDeque::new();
        let mut visited: HashSet<FileId> = HashSet::new();

        worklist.push_back(entry_file);
        
        while let Some(file_id) = worklist.pop_front() {
            if !visited.insert(file_id) {
                // 如果文件已经被访问过（insert 返回 false），就跳过
                continue;
            }

            // 1. 通过 Trait 向“数据库”查询这个文件的 AST
            let ast = self.db.ast(file_id);

            // 2. 调用我们的单文件分析函数，进行定义收集
            self.collect_defs_in_module(&ast);

            // 3. 扫描 `use` 语句，发现新的依赖文件
            for item in &ast.items {
                if let AstItem::Use(use_stmt) = item {
                    self.discover_deps_in_tree(file_id, &use_stmt.tree, &mut worklist);
                }
            }
        }
        
        self.def_map // 返回最终的成果
    }
    
    /// (这是一个私有辅助函数) 负责扫描单个模块的 AST，并将定义添加到 DefMap。
    fn collect_defs_in_module(&mut self, module_ast: &ast::Module) {
        for item in &module_ast.items {
            match &item {
                AstItem::Function(func_def) => {
                    let def_id = self.id_allocator.new_def_id();
                    
                    // --- 核心修复点 ---
                    // 通过 db 接口调用 interner 服务，将 &str 转换为 Symbol
                    let name_symbol = self.db.intern_string(&func_def.name.lexeme);

                    let item_def = ItemDef {
                        def_id,
                        name: name_symbol, // 现在类型匹配了！
                        kind: ItemKind::Function,
                        ast_node: Arc::new(item.clone()),
                    };
                    self.def_map.items.insert(def_id, item_def);
                }
                AstItem::Struct(struct_def) => {
                    let def_id = self.id_allocator.new_def_id();
                    
                    // --- 核心修复点 ---
                    let name_symbol = self.db.intern_string(&struct_def.name.lexeme);
                    
                    let item_def = ItemDef {
                        def_id,
                        name: name_symbol, // 类型匹配！
                        kind: ItemKind::Struct,
                        ast_node: Arc::new(item.clone()),
                    };
                    self.def_map.items.insert(def_id, item_def);
                }
                AstItem::Use(_) => { /* ... */ }
            }
        }
    }

    /// (新的私有辅助函数) 递归地遍历 UseTree，找出所有需要解析的模块路径
    fn discover_deps_in_tree(&self, anchor_file: FileId, tree: &ast::UseTree, worklist: &mut VecDeque<FileId>) {
        match tree {
            ast::UseTree::Simple { path, .. } => {
                // 通过 Trait，让“数据库”去解析这个 use 路径
                if let Some(resolved_file_id) = self.db.resolve_module(anchor_file, path) {
                    worklist.push_back(resolved_file_id);
                }
            },
            ast::UseTree::Group { items } => {
                // 递归地处理分组中的每一项
                for item in items {
                    self.discover_deps_in_tree(anchor_file, item, worklist);
                }
            },
            ast::UseTree::Wildcard { .. } => {
                // 通配符导入也需要解析其路径
                // 注意：我们的 UseStmt AST 设计需要微调来支持 `use a::b::*`
                // 暂时先忽略
            }
        }
    }

}