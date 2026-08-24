use crate::analysis::DocumentAnalysis;
use crate::types::Type;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, InsertTextFormat, Position,
};

pub fn get_completions(
    analysis: &DocumentAnalysis,
    position: Position,
) -> Option<CompletionResponse> {
    let line_idx = position.line as usize;
    let col_idx = position.character as usize;

    let line = analysis.source.lines().nth(line_idx).unwrap_or("");
    let line_prefix = if col_idx <= line.len() {
        &line[..col_idx]
    } else {
        line
    };

    let mut items = Vec::new();

    // Check for member access `obj.`
    if let Some(dot_idx) = line_prefix.rfind('.') {
        let before_dot = line_prefix[..dot_idx].trim();
        // Simple identifier before dot
        let target_var = before_dot
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .last()
            .unwrap_or("")
            .to_string();

        let target_ty = if target_var == "self" {
            // Find current struct
            analysis
                .structs
                .keys()
                .next()
                .map(|s| Type::StructInstance(s.clone()))
                .unwrap_or(Type::Unknown)
        } else if let Some(sym) = analysis.vars.get(&target_var) {
            sym.ty.clone()
        } else if target_var.starts_with('"') || before_dot.ends_with('"') {
            Type::String
        } else if target_var.starts_with('[') || before_dot.ends_with(']') {
            Type::List(None)
        } else if let Some(st) = analysis.structs.get(&target_var) {
            Type::StructDef(st.name.clone())
        } else {
            Type::Unknown
        };

        match target_ty {
            Type::String => {
                items.extend(get_string_method_completions());
            }
            Type::List(_) => {
                items.extend(get_list_method_completions());
            }
            Type::StructInstance(ref sname) => {
                if let Some(st) = analysis.structs.get(sname) {
                    for (field_name, field_ty) in &st.fields {
                        items.push(CompletionItem {
                            label: field_name.clone(),
                            kind: Some(CompletionItemKind::FIELD),
                            detail: Some(format!("field: {}", field_ty)),
                            documentation: None,
                            ..Default::default()
                        });
                    }
                    for (method_name, (params, ret_ty)) in &st.methods {
                        items.push(CompletionItem {
                            label: format!("{}({})", method_name, params.join(", ")),
                            insert_text: Some(format!("{}({})", method_name, snippet_args(params))),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            kind: Some(CompletionItemKind::METHOD),
                            detail: Some(format!("fn({}) -> {}", params.join(", "), ret_ty)),
                            documentation: None,
                            ..Default::default()
                        });
                    }
                }
            }
            Type::File => {
                items.extend(get_file_method_completions());
            }
            Type::Socket => {
                items.extend(get_socket_method_completions());
            }
            Type::Server => {
                items.extend(get_server_method_completions());
            }
            _ => {
                // Default fallback member completions if type unknown
                items.extend(get_string_method_completions());
                items.extend(get_list_method_completions());
            }
        }

        return Some(CompletionResponse::Array(items));
    }

    // Check for namespace access `mod::`
    if let Some(dcolon_idx) = line_prefix.rfind("::") {
        let before_dcolon = line_prefix[..dcolon_idx].trim();
        let target_mod = before_dcolon
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .last()
            .unwrap_or("");

        match target_mod {
            "io" => items.extend(get_io_module_completions()),
            "fs" => items.extend(get_fs_module_completions()),
            "net" => items.extend(get_net_module_completions()),
            "ffi" => items.extend(get_ffi_module_completions()),
            _ => {
                if let Some(st) = analysis.structs.get(target_mod) {
                    for (method_name, (params, ret_ty)) in &st.methods {
                        items.push(CompletionItem {
                            label: format!("{}({})", method_name, params.join(", ")),
                            insert_text: Some(format!("{}({})", method_name, snippet_args(params))),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            kind: Some(CompletionItemKind::FUNCTION),
                            detail: Some(format!("fn({}) -> {}", params.join(", "), ret_ty)),
                            documentation: None,
                            ..Default::default()
                        });
                    }
                }
            }
        }

        return Some(CompletionResponse::Array(items));
    }

    // General completion (variables with known inferred type, functions, structs, keywords)
    for (var_name, sym) in &analysis.vars {
        items.push(CompletionItem {
            label: var_name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(format!("type: {}", sym.ty)),
            documentation: None,
            ..Default::default()
        });
    }

    for (fn_name, func) in &analysis.functions {
        items.push(CompletionItem {
            label: format!("{}({})", fn_name, func.params.join(", ")),
            insert_text: Some(format!("{}({})", fn_name, snippet_args(&func.params))),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(format!("fn({}) -> {}", func.params.join(", "), func.ret_ty)),
            documentation: None,
            ..Default::default()
        });
    }

    for (struct_name, st) in &analysis.structs {
        items.push(CompletionItem {
            label: struct_name.clone(),
            kind: Some(CompletionItemKind::STRUCT),
            detail: Some(format!("struct {}", st.name)),
            documentation: None,
            ..Default::default()
        });
    }

    // Builtin functions & Standard modules
    items.extend(get_builtin_function_completions());
    items.extend(get_keyword_completions());

    Some(CompletionResponse::Array(items))
}

fn snippet_args(params: &[String]) -> String {
    params
        .iter()
        .enumerate()
        .map(|(i, p)| format!("${{{}:{}}}", i + 1, p))
        .collect::<Vec<_>>()
        .join(", ")
}

fn get_string_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "len()".to_string(),
            insert_text: Some("len()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> number".to_string()),
            documentation: Some(Documentation::String("Returns string length in bytes".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "substr(start, len)".to_string(),
            insert_text: Some("substr(${1:start}, ${2:len})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(start, len) -> string".to_string()),
            documentation: Some(Documentation::String("Extracts substring".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "split(sep)".to_string(),
            insert_text: Some("split(${1:sep})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(sep) -> list[string]".to_string()),
            documentation: Some(Documentation::String("Splits string by separator".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "ord(index)".to_string(),
            insert_text: Some("ord(${1:index})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(index) -> number".to_string()),
            documentation: Some(Documentation::String("Returns ASCII code at index".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "trim()".to_string(),
            insert_text: Some("trim()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            documentation: Some(Documentation::String("Trims surrounding whitespace".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "to_upper()".to_string(),
            insert_text: Some("to_upper()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            documentation: Some(Documentation::String("Converts string to uppercase".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "to_lower()".to_string(),
            insert_text: Some("to_lower()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            documentation: Some(Documentation::String("Converts string to lowercase".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "to_number()".to_string(),
            insert_text: Some("to_number()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> number".to_string()),
            documentation: Some(Documentation::String("Parses string as number".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "replace(from, to)".to_string(),
            insert_text: Some("replace(${1:from}, ${2:to})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(from, to) -> string".to_string()),
            documentation: Some(Documentation::String("Replaces occurrences of substring".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "includes(search)".to_string(),
            insert_text: Some("includes(${1:search})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(search) -> bool".to_string()),
            documentation: Some(Documentation::String("Checks if string contains search text".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "push(str)".to_string(),
            insert_text: Some("push(${1:str})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(str) -> null".to_string()),
            documentation: Some(Documentation::String("Appends text to string".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "pop()".to_string(),
            insert_text: Some("pop()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            documentation: Some(Documentation::String("Pops last character".into())),
            ..Default::default()
        },
    ]
}

fn get_list_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "len()".to_string(),
            insert_text: Some("len()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> number".to_string()),
            documentation: Some(Documentation::String("Returns length of list".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "push(item)".to_string(),
            insert_text: Some("push(${1:item})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(item) -> null".to_string()),
            documentation: Some(Documentation::String("Appends item to list".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "pop()".to_string(),
            insert_text: Some("pop()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> item".to_string()),
            documentation: Some(Documentation::String("Pops last item from list".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "join(sep)".to_string(),
            insert_text: Some("join(${1:sep})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(sep) -> string".to_string()),
            documentation: Some(Documentation::String("Joins list elements into string".into())),
            ..Default::default()
        },
    ]
}

fn get_file_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "read()".to_string(),
            insert_text: Some("read()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "read_range(start, size)".to_string(),
            insert_text: Some("read_range(${1:start}, ${2:size})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(start, size) -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "write(data)".to_string(),
            insert_text: Some("write(${1:data})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(data) -> bool".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "write_at(data, start)".to_string(),
            insert_text: Some("write_at(${1:data}, ${2:start})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(data, start) -> bool".to_string()),
            ..Default::default()
        },
    ]
}

fn get_socket_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "read()".to_string(),
            insert_text: Some("read()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "read_bytes(len)".to_string(),
            insert_text: Some("read_bytes(${1:len})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(len) -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "write(data)".to_string(),
            insert_text: Some("write(${1:data})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn(data) -> bool".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "close()".to_string(),
            insert_text: Some("close()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> bool".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "is_connected()".to_string(),
            insert_text: Some("is_connected()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> bool".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "local_addr()".to_string(),
            insert_text: Some("local_addr()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "peer_addr()".to_string(),
            insert_text: Some("peer_addr()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "read_all()".to_string(),
            insert_text: Some("read_all()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> string".to_string()),
            ..Default::default()
        },
    ]
}

fn get_server_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "accept()".to_string(),
            insert_text: Some("accept()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> Socket".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "close()".to_string(),
            insert_text: Some("close()".to_string()),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("fn() -> bool".to_string()),
            ..Default::default()
        },
    ]
}

fn get_io_module_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "print(str)".to_string(),
            insert_text: Some("print(${1:str})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(str) -> null".to_string()),
            documentation: Some(Documentation::String("Prints formatted string to stdout".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "println(str)".to_string(),
            insert_text: Some("println(${1:str})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(str) -> null".to_string()),
            documentation: Some(Documentation::String("Prints formatted string + newline to stdout".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "input(prompt)".to_string(),
            insert_text: Some("input(${1:prompt})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(prompt=\"\") -> string".to_string()),
            documentation: Some(Documentation::String("Prompts user and reads line from stdin".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "format(str)".to_string(),
            insert_text: Some("format(${1:str})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(str) -> string".to_string()),
            documentation: Some(Documentation::String("Interpolates {var} expressions in string".into())),
            ..Default::default()
        },
    ]
}

fn get_fs_module_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "open(path)".to_string(),
            insert_text: Some("open(${1:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path) -> File".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "create(path)".to_string(),
            insert_text: Some("create(${1:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path) -> File".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "list_dir(path)".to_string(),
            insert_text: Some("list_dir(${1:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path) -> list[string]".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "exists(path)".to_string(),
            insert_text: Some("exists(${1:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path) -> bool".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "delete(path)".to_string(),
            insert_text: Some("delete(${1:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path) -> bool".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "append(path, data)".to_string(),
            insert_text: Some("append(${1:path}, ${2:data})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path, data) -> bool".to_string()),
            ..Default::default()
        },
    ]
}

fn get_net_module_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "connect(address, port)".to_string(),
            insert_text: Some("connect(${1:address}, ${2:port})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(address, port) -> Socket".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "bind(address, port)".to_string(),
            insert_text: Some("bind(${1:address}, ${2:port})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(address, port) -> Server".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "http_get(host, port, path)".to_string(),
            insert_text: Some("http_get(${1:host}, ${2:port}, ${3:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(host, port, path) -> string".to_string()),
            ..Default::default()
        },
    ]
}

fn get_ffi_module_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "load(path)".to_string(),
            insert_text: Some("load(${1:path})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(path) -> lib".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "sym(lib, name, sig)".to_string(),
            insert_text: Some("sym(${1:lib}, ${2:name}, ${3:sig})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(lib, name, sig) -> ForeignFn".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "def_struct(name, spec)".to_string(),
            insert_text: Some("def_struct(${1:name}, ${2:spec})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(name, spec) -> string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "struct_val(name, vals)".to_string(),
            insert_text: Some("struct_val(${1:name}, ${2:vals})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(name, vals) -> CStruct".to_string()),
            ..Default::default()
        },
    ]
}

fn get_builtin_function_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "write(val)".to_string(),
            insert_text: Some("write(${1:val})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(val) -> null".to_string()),
            documentation: Some(Documentation::String("Writes string or value to stdout".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "exit(code)".to_string(),
            insert_text: Some("exit(${1:code})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(code) -> null".to_string()),
            documentation: Some(Documentation::String("Exits current process with code".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "chr(code)".to_string(),
            insert_text: Some("chr(${1:code})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(code) -> string".to_string()),
            documentation: Some(Documentation::String("Returns ASCII char from number code".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "readline()".to_string(),
            insert_text: Some("readline()".to_string()),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn() -> string".to_string()),
            documentation: Some(Documentation::String("Reads line from stdin".into())),
            ..Default::default()
        },
        CompletionItem {
            label: "eval(code)".to_string(),
            insert_text: Some("eval(${1:code})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn(code) -> value".to_string()),
            documentation: Some(Documentation::String("Evaluates code string in interpreter".into())),
            ..Default::default()
        },
    ]
}

fn get_keyword_completions() -> Vec<CompletionItem> {
    let kw = vec![
        "let", "fn", "struct", "ret", "if", "els", "elsif", "while", "for", "match", "break",
        "continue", "import", "as", "true", "false", "null", "self",
    ];
    kw.into_iter()
        .map(|k| CompletionItem {
            label: k.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect()
}
