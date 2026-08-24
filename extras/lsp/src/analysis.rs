use crate::types::Type;
use iok::interner::resolve;
use iok::lexer::{Lexer, Loc};
use iok::parser::{Node, Parser, Tree};
use lsp_types::{Diagnostic, Position, Range};
use std::collections::HashMap;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SymbolInfo {
    pub name: String,
    pub ty: Type,
    pub loc: Loc,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub fields: HashMap<String, Type>,
    pub methods: HashMap<String, (Vec<String>, Type)>,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub params: Vec<String>,
    pub ret_ty: Type,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocumentAnalysis {
    pub source: String,
    pub ast: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
    pub vars: HashMap<String, SymbolInfo>,
    pub structs: HashMap<String, StructInfo>,
    pub functions: HashMap<String, FunctionInfo>,
    pub imports: HashMap<String, String>, // alias -> path
}

impl DocumentAnalysis {
    pub fn new(source: &str) -> Self {
        let source_str = source.to_string();
        let mut lexer = Lexer::new(&source_str);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens.clone());
        let ast = parser.parse_tokens();

        let diagnostics = Vec::new();

        // Standard library / built-in known structs
        let mut structs = HashMap::new();
        structs.insert(
            "File".to_string(),
            StructInfo {
                name: "File".to_string(),
                fields: [("path".to_string(), Type::String)].into_iter().collect(),
                methods: [
                    ("read".to_string(), (vec![], Type::String)),
                    (
                        "read_range".to_string(),
                        (vec!["start".to_string(), "size".to_string()], Type::String),
                    ),
                    (
                        "write".to_string(),
                        (vec!["data".to_string()], Type::Bool),
                    ),
                    (
                        "write_at".to_string(),
                        (
                            vec!["data".to_string(), "start".to_string()],
                            Type::Bool,
                        ),
                    ),
                ]
                .into_iter()
                .collect(),
                loc: Loc { x: 1, y: 1, src: 0 },
            },
        );

        structs.insert(
            "Socket".to_string(),
            StructInfo {
                name: "Socket".to_string(),
                fields: HashMap::new(),
                methods: [
                    ("read".to_string(), (vec![], Type::String)),
                    (
                        "read_bytes".to_string(),
                        (vec!["len".to_string()], Type::String),
                    ),
                    (
                        "write".to_string(),
                        (vec!["data".to_string()], Type::Bool),
                    ),
                    ("close".to_string(), (vec![], Type::Bool)),
                    ("is_connected".to_string(), (vec![], Type::Bool)),
                    ("local_addr".to_string(), (vec![], Type::String)),
                    ("peer_addr".to_string(), (vec![], Type::String)),
                    ("read_all".to_string(), (vec![], Type::String)),
                ]
                .into_iter()
                .collect(),
                loc: Loc { x: 1, y: 1, src: 0 },
            },
        );

        structs.insert(
            "Server".to_string(),
            StructInfo {
                name: "Server".to_string(),
                fields: HashMap::new(),
                methods: [
                    (
                        "accept".to_string(),
                        (vec![], Type::StructInstance("Socket".to_string())),
                    ),
                    ("close".to_string(), (vec![], Type::Bool)),
                ]
                .into_iter()
                .collect(),
                loc: Loc { x: 1, y: 1, src: 0 },
            },
        );

        let mut analysis = DocumentAnalysis {
            source: source.to_string(),
            ast,
            diagnostics,
            vars: HashMap::new(),
            structs,
            functions: HashMap::new(),
            imports: HashMap::new(),
        };

        analysis.analyze();
        analysis
    }

    fn analyze(&mut self) {
        // First pass: collect struct defs and function defs
        for node in &self.ast {
            match &node.tree {
                Tree::StructDef {
                    name,
                    fields,
                    methods,
                } => {
                    let struct_name = resolve(*name);
                    let mut field_map = HashMap::new();
                    for field in fields {
                        if let Tree::Let(fname, init) = &field.tree {
                            let fn_str = resolve(*fname);
                            let fty = self.infer_type(init);
                            field_map.insert(fn_str, fty);
                        }
                    }

                    let mut method_map = HashMap::new();
                    for method in methods {
                        if let Tree::Fn {
                            name: Some(mname),
                            args,
                            body,
                        } = &method.tree
                        {
                            let method_name = resolve(*mname);
                            let params = self.extract_param_names(args);
                            let ret_ty = self.infer_body_return_type(body);
                            method_map.insert(method_name, (params, ret_ty));
                        }
                    }

                    self.structs.insert(
                        struct_name.clone(),
                        StructInfo {
                            name: struct_name,
                            fields: field_map,
                            methods: method_map,
                            loc: node.loc,
                        },
                    );
                }
                Tree::Fn { name: Some(n), args, body } => {
                    let fn_name = resolve(*n);
                    let params = self.extract_param_names(args);
                    let ret_ty = self.infer_body_return_type(body);
                    self.functions.insert(
                        fn_name.clone(),
                        FunctionInfo {
                            name: fn_name,
                            params,
                            ret_ty,
                            loc: node.loc,
                        },
                    );
                }
                Tree::Import { path, alias } => {
                    if let Tree::String(p) = &path.tree {
                        let path_str = p.to_string();
                        let alias_str = alias.map(resolve).unwrap_or_else(|| {
                            path_str.split('/').last().unwrap_or(&path_str).replace(".iok", "")
                        });
                        self.imports.insert(alias_str, path_str);
                    } else if let Tree::MemberAccess { target, member } = &path.tree {
                        // e.g. import std::io
                        if let (Tree::Ident(t), Tree::Ident(m)) = (&target.tree, &member.tree) {
                            let mod_name = format!("{}::{}", resolve(*t), resolve(*m));
                            let alias_str = alias.map(resolve).unwrap_or_else(|| resolve(*m));
                            self.imports.insert(alias_str, mod_name);
                        }
                    }
                }
                _ => {}
            }
        }

        // Second pass: variables and scope declarations
        for node in &self.ast {
            match &node.tree {
                Tree::Let(var_id, init) => {
                    let var_name = resolve(*var_id);
                    let ty = self.infer_type(init);
                    self.vars.insert(
                        var_name.clone(),
                        SymbolInfo {
                            name: var_name,
                            ty,
                            loc: node.loc,
                            doc: None,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    pub fn extract_param_names(&self, args: &[Node]) -> Vec<String> {
        let mut names = Vec::new();
        for arg in args {
            match &arg.tree {
                Tree::Ident(id) => names.push(resolve(*id)),
                Tree::Let(id, _) => names.push(resolve(*id)),
                Tree::Assign(lhs, _) => {
                    if let Tree::Ident(id) = &lhs.tree {
                        names.push(resolve(*id));
                    }
                }
                _ => {}
            }
        }
        names
    }

    pub fn infer_body_return_type(&self, body: &[Node]) -> Type {
        for node in body {
            if let Tree::Ret(expr) = &node.tree {
                return self.infer_type(expr);
            }
            if let Tree::If { body: if_b, els, els_ifs: _, .. } = &node.tree {
                let t = self.infer_body_return_type(if_b);
                if t != Type::Unknown {
                    return t;
                }
                let t_els = self.infer_body_return_type(els);
                if t_els != Type::Unknown {
                    return t_els;
                }
            }
        }
        Type::Unknown
    }

    pub fn infer_type(&self, node: &Node) -> Type {
        match &node.tree {
            Tree::Number(_) => Type::Number,
            Tree::String(_) => Type::String,
            Tree::Bool(_) => Type::Bool,
            Tree::Range(_, _) => Type::Range,
            Tree::Empty() => Type::Null,
            Tree::List(items) => {
                let elem_ty = items.first().map(|i| Box::new(self.infer_type(i)));
                Type::List(elem_ty)
            }
            Tree::Ident(id) => {
                let name = resolve(*id);
                if name == "true" || name == "false" {
                    return Type::Bool;
                }
                if name == "null" {
                    return Type::Null;
                }
                if let Some(sym) = self.vars.get(&name) {
                    return sym.ty.clone();
                }
                if let Some(st) = self.structs.get(&name) {
                    return Type::StructDef(st.name.clone());
                }
                Type::Unknown
            }
            Tree::StructInit { name, .. } => {
                let sname = resolve(*name);
                Type::StructInstance(sname)
            }
            Tree::BinOp(left, _op, right) => {
                let lty = self.infer_type(left);
                let rty = self.infer_type(right);
                if lty == Type::String || rty == Type::String {
                    Type::String
                } else if lty == Type::Number && rty == Type::Number {
                    Type::Number
                } else {
                    Type::Unknown
                }
            }
            Tree::CmpOp(_, _, _) => Type::Bool,
            Tree::FnCall { name, args: _ } => {
                let fname = resolve(*name);
                if fname == "chr" || fname == "readline" {
                    return Type::String;
                }
                if fname == "write" || fname == "exit" {
                    return Type::Null;
                }
                if let Some(f) = self.functions.get(&fname) {
                    return f.ret_ty.clone();
                }
                Type::Unknown
            }
            Tree::Fn { args, body, .. } => {
                let params = self.extract_param_names(args);
                let ret = Box::new(self.infer_body_return_type(body));
                Type::Function { params, ret }
            }
            Tree::MemberAccess { target, member } => {
                let target_ty = self.infer_type(target);
                let member_name = match &member.tree {
                    Tree::Ident(id) => resolve(*id),
                    Tree::FnCall { name, .. } => resolve(*name),
                    _ => String::new(),
                };

                match target_ty {
                    Type::String => match member_name.as_str() {
                        "len" | "ord" | "to_number" => Type::Number,
                        "substr" | "trim" | "to_upper" | "to_lower" | "replace" => Type::String,
                        "includes" => Type::Bool,
                        "split" => Type::List(Some(Box::new(Type::String))),
                        _ => Type::Unknown,
                    },
                    Type::List(elem_ty) => match member_name.as_str() {
                        "len" => Type::Number,
                        "join" => Type::String,
                        "pop" => elem_ty.map(|b| *b).unwrap_or(Type::Unknown),
                        _ => Type::Unknown,
                    },
                    Type::StructInstance(sname) => {
                        if let Some(st) = self.structs.get(&sname) {
                            if let Some(fty) = st.fields.get(&member_name) {
                                return fty.clone();
                            }
                            if let Some((_, mret)) = st.methods.get(&member_name) {
                                return mret.clone();
                            }
                        }
                        Type::Unknown
                    }
                    Type::StructDef(sname) => {
                        if member_name == "new" {
                            return Type::StructInstance(sname);
                        }
                        if let Some(st) = self.structs.get(&sname) {
                            if let Some((_, mret)) = st.methods.get(&member_name) {
                                return mret.clone();
                            }
                        }
                        Type::Unknown
                    }
                    Type::Module(mname) => match mname.as_str() {
                        "io" => match member_name.as_str() {
                            "format" | "input" => Type::String,
                            _ => Type::Null,
                        },
                        "fs" => match member_name.as_str() {
                            "open" | "create" => Type::StructInstance("File".to_string()),
                            "read" => Type::String,
                            "exists" => Type::Bool,
                            "list_dir" => Type::List(Some(Box::new(Type::String))),
                            _ => Type::Unknown,
                        },
                        "net" => match member_name.as_str() {
                            "connect" => Type::StructInstance("Socket".to_string()),
                            "bind" => Type::StructInstance("Server".to_string()),
                            "http_get" => Type::String,
                            _ => Type::Unknown,
                        },
                        _ => Type::Unknown,
                    },
                    _ => Type::Unknown,
                }
            }
            _ => Type::Unknown,
        }
    }
}

pub fn loc_to_range(loc: Loc) -> Range {
    let line = loc.y.saturating_sub(1) as u32;
    let col = loc.x.saturating_sub(1) as u32;
    Range {
        start: Position { line, character: col },
        end: Position { line, character: col + 1 },
    }
}
