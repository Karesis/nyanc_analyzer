use super::super::*;
use crate::resolver::ItemKind;
use ast::{Module as AstModule, Path as AstPath};
use nyanc_core::{FileId, Symbol};
use parser::Parser;
use reporter::DiagnosticsEngine;
use lexer::Lexer;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::collections::HashSet;

// --- 步骤 1: 在测试模块内部，定义一个自给自足的 Interner ---
// 这个 Interner 的定义和实现与 driver 中的完全一样，但它只属于这个测试模块。
#[derive(Debug, Default)]
struct TestInterner {
    map: HashMap<String, Symbol>,
    vec: Vec<String>,
}

impl TestInterner {
    fn intern(&mut self, s: &str) -> Symbol {
        if let Some(symbol) = self.map.get(s) {
            return *symbol;
        }
        let symbol = Symbol(self.vec.len() as u32);
        let s = s.to_string();
        self.vec.push(s.clone());
        self.map.insert(s, symbol);
        symbol
    }

    /// 根据 Symbol 查找回原始的字符串切片。
    fn lookup(&self, symbol: Symbol) -> &str {
        // Symbol 的 u32 值，就是它在 vec 中的索引
        &self.vec[symbol.0 as usize]
    }
}

// --- 步骤 2: 更新 MockDb，让它使用我们本地的 TestInterner ---
#[derive(Default)]
struct MockDb {
    interner: RefCell<TestInterner>, // <-- 使用 TestInterner
    sources: HashMap<FileId, Arc<String>>,
    paths: HashMap<String, FileId>,
    ast_cache: RefCell<HashMap<FileId, Arc<AstModule>>>,
}

impl AnalyzerDb for MockDb {
    fn ast(&self, file_id: FileId) -> Arc<AstModule> {
        if let Some(ast) = self.ast_cache.borrow().get(&file_id) {
            return ast.clone();
        }

        let source_text = self.sources.get(&file_id).unwrap().clone();
        let diagnostics = DiagnosticsEngine::default(); // 测试中暂时忽略解析错误
        let lexer = Lexer::new(&source_text, file_id, &diagnostics);
        let mut parser = Parser::new(lexer, &diagnostics);
        let ast = Arc::new(parser.parse());
        
        self.ast_cache.borrow_mut().insert(file_id, ast.clone());
        ast
    }
    
    // 模拟模块解析：只处理简单的文件名
    fn resolve_module(&self, _anchor_file: FileId, path: &AstPath) -> Option<FileId> {
        let path_str = path.segments.iter()
            .map(|s| s.lexeme.as_str())
            .collect::<Vec<_>>()
            .join("/");
        
        // --- 核心修复点 ---
        // 模拟真实的模块解析行为：尝试添加 .ny 后缀
        let mut resolved_path = path_str;
        if !resolved_path.ends_with(".ny") {
             resolved_path.push_str(".ny");
        }

        self.paths.get(&resolved_path).copied()
    }
    
    fn intern_string(&self, s: &str) -> Symbol {
        self.interner.borrow_mut().intern(s)
    }
}


impl MockDb {
    fn add_file(&mut self, path: &str, source: &str) -> FileId {
        let file_id: FileId = self.sources.len();
        self.sources.insert(file_id, Arc::new(source.to_string()));
        self.paths.insert(path.to_string(), file_id);
        file_id
    }
}

// --- 步骤 3: 我们的测试用例现在可以无依赖地运行了 ---
#[test]
fn test_multi_module_def_collection() {
    // 1. 准备 (Arrange): 创建一个模拟的多文件项目
    let mut db = MockDb::default();
    
    let main_source = r#"
        use utils
        fun main() {}
    "#;
    let utils_source = r#"
        struct Point {}
        fun helper() {}
    "#;

    let main_fid = db.add_file("main.ny", main_source);
    let _utils_fid = db.add_file("utils.ny", utils_source);

    // 2. 执行 (Act): 运行我们的定义收集器
    let resolver = Resolver::new(&db);
    let def_map = resolver.collect_defs_crate(main_fid);

    // 3. 断言 (Assert): 检查结果是否符合预期
    
    // a. 应该找到了 3 个顶层定义
    assert_eq!(def_map.items.len(), 3);

    // b. 我们可以检查找到的定义的名字是否正确
    let found_names: HashSet<String> = def_map.items.values()
        .map(|item_def| {
            // --- 核心修复点 A ---
            // 使用 db (我们的 MockDb) 来查询 Symbol 对应的字符串
            db.interner.borrow().lookup(item_def.name).to_string()
        })
        .collect();
        
    let expected_names: HashSet<String> = ["main", "Point", "helper"].iter()
        .map(|s| s.to_string())
        .collect();
    
    assert_eq!(found_names, expected_names);

    // c. 我们甚至可以进一步检查每个定义的种类
    for item in def_map.items.values() {
        // match 的对象是翻译回来的 &str
        match db.interner.borrow().lookup(item.name) {
            "main" => assert_eq!(item.kind, ItemKind::Function),
            "Point" => assert_eq!(item.kind, ItemKind::Struct),
            "helper" => assert_eq!(item.kind, ItemKind::Function),
            // 为了让 panic 信息更友好，我们也翻译一下
            unexpected_name => panic!("Unexpected item found: {:?}", unexpected_name),
        }
    }
}