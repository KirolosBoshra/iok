use crate::analysis::DocumentAnalysis;
use iok::parser::Tree;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

pub fn get_hover(analysis: &DocumentAnalysis, position: Position) -> Option<Hover> {
    let line_idx = position.line as usize;
    let col_idx = position.character as usize;

    let line = analysis.source.lines().nth(line_idx)?;
    let word = get_word_at_position(line, col_idx)?;

    // Check variables
    if let Some(sym) = analysis.vars.get(&word) {
        let content = format!("```iok\nlet {}: {}\n```", sym.name, sym.ty);
        return Some(make_hover(&content));
    }

    // Check functions
    if let Some(func) = analysis.functions.get(&word) {
        let content = format!(
            "```iok\nfn {}({}) -> {}\n```",
            func.name,
            func.params.join(", "),
            func.ret_ty
        );
        return Some(make_hover(&content));
    }

    // Check structs
    if let Some(st) = analysis.structs.get(&word) {
        let mut fields_str = Vec::new();
        for (f, ty) in &st.fields {
            fields_str.push(format!("  let {}: {}", f, ty));
        }
        let mut methods_str = Vec::new();
        for (m, (params, rty)) in &st.methods {
            methods_str.push(format!("  fn {}({}) -> {}", m, params.join(", "), rty));
        }
        let content = format!(
            "```iok\nstruct {} {{\n{}\n{}\n}}\n```",
            st.name,
            fields_str.join("\n"),
            methods_str.join("\n")
        );
        return Some(make_hover(&content));
    }

    // Check standard library modules (io, fs, net, ffi)
    if let Some(content) = get_module_function_hover(&word, &analysis) {
        return Some(make_hover(&content));
    }

    // Check qualified paths: io::print, Person::new, etc.
    if let Some(content) = get_qualified_hover(line, col_idx, &analysis) {
        return Some(make_hover(&content));
    }

    // Check anonymous functions at cursor position
    if let Some(content) = get_anonymous_fn_hover(&analysis, line_idx, col_idx) {
        return Some(make_hover(&content));
    }

    None
}

fn make_hover(content: &str) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content.to_string(),
        }),
        range: None,
    }
}

fn get_module_function_hover(word: &str, _analysis: &DocumentAnalysis) -> Option<String> {
    let std_fn_docs: &[(&str, &str, &str)] = &[
        ("write", "fn(val) -> null", "Writes string or value to stdout"),
        ("exit", "fn(code) -> null", "Exits current process"),
        ("chr", "fn(code) -> string", "Returns ASCII char from number"),
        ("readline", "fn() -> string", "Reads line from stdin"),
        ("eval", "fn(code) -> value", "Evaluates code string"),
    ];

    // Check if it's a builtin
    for (name, sig, doc) in std_fn_docs {
        if word == *name {
            return Some(format!("```iok\n{}\n```\n{}", sig, doc));
        }
    }

    // Check if it's a known module function from imports
    // e.g., user has `import std::io`, cursor on `println` — not useful without qualifier
    None
}

fn get_qualified_hover(line: &str, col_idx: usize, analysis: &DocumentAnalysis) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    if col_idx >= chars.len() {
        return None;
    }

    // Find word start
    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_';
    let mut word_start = col_idx;
    while word_start > 0 && is_ident_char(chars[word_start - 1]) {
        word_start -= 1;
    }

    // Check if :: is before the word
    if word_start < 2 {
        return None;
    }
    if chars[word_start - 1] != ':' || chars[word_start - 2] != ':' {
        return None;
    }

    // Find qualifier before ::
    let mut qual_end = word_start - 3;
    while qual_end > 0 && is_ident_char(chars[qual_end - 1]) {
        qual_end -= 1;
    }
    if qual_end >= word_start - 2 {
        return None;
    }

    let qualifier: String = chars[qual_end..word_start - 2].iter().collect();
    let method: String = chars[word_start..].iter().take_while(|c| is_ident_char(**c)).collect();

    // Check io module functions
    if qualifier == "io" {
        let io_fns: &[(&str, &str)] = &[
            ("print", "fn(str) -> null\nPrints formatted string to stdout"),
            ("println", "fn(str) -> null\nPrints formatted string + newline to stdout"),
            ("input", "fn(prompt=\"\") -> string\nPrompts user and reads line from stdin"),
            ("format", "fn(str) -> string\nInterpolates {var} expressions in string"),
        ];
        for (name, doc) in io_fns {
            if method == *name {
                return Some(format!("```iok\n{}\n```", doc.replace('\n', "\n```iok\n")));
            }
        }
    }

    // Check fs module functions
    if qualifier == "fs" {
        let fs_fns: &[(&str, &str)] = &[
            ("open", "fn(path) -> File\nOpens a file for reading/writing"),
            ("create", "fn(path) -> File\nCreates a new file"),
            ("list_dir", "fn(path) -> list[string]\nLists directory contents"),
            ("exists", "fn(path) -> bool\nChecks if path exists"),
            ("delete", "fn(path) -> bool\nDeletes a file"),
            ("append", "fn(path, data) -> bool\nAppends data to a file"),
        ];
        for (name, doc) in fs_fns {
            if method == *name {
                return Some(format!("```iok\n{}\n```", doc.replace('\n', "\n```iok\n")));
            }
        }
    }

    // Check net module functions
    if qualifier == "net" {
        let net_fns: &[(&str, &str)] = &[
            ("connect", "fn(address, port) -> Socket\nConnects to a remote server"),
            ("bind", "fn(address, port) -> Server\nBinds to an address and listens"),
            ("http_get", "fn(host, port, path) -> string\nPerforms a simple HTTP GET request"),
        ];
        for (name, doc) in net_fns {
            if method == *name {
                return Some(format!("```iok\n{}\n```", doc.replace('\n', "\n```iok\n")));
            }
        }
    }

    // Check ffi module functions
    if qualifier == "ffi" {
        let ffi_fns: &[(&str, &str)] = &[
            ("load", "fn(path) -> lib\nLoads a shared library"),
            ("sym", "fn(lib, name, sig) -> ForeignFn\nGets a symbol from a loaded library"),
            ("def_struct", "fn(name, spec) -> string\nDefines a C struct layout"),
            ("struct_val", "fn(name, vals) -> CStruct\nCreates a struct value"),
        ];
        for (name, doc) in ffi_fns {
            if method == *name {
                return Some(format!("```iok\n{}\n```", doc.replace('\n', "\n```iok\n")));
            }
        }
    }

    // Check Struct::method (e.g., Person::new)
    if let Some(st) = analysis.structs.get(&qualifier) {
        if method == "new" {
            let content = format!(
                "```iok\n{}::new({}) -> {}\n```",
                st.name,
                st.fields.keys().cloned().collect::<Vec<_>>().join(", "),
                st.name
            );
            return Some(content);
        }
        if let Some((params, rty)) = st.methods.get(&method) {
            let content = format!(
                "```iok\nfn {}({}) -> {}\n```",
                method,
                params.join(", "),
                rty
            );
            return Some(content);
        }
    }

    None
}

fn get_anonymous_fn_hover(analysis: &DocumentAnalysis, line_idx: usize, col_idx: usize) -> Option<String> {
    let target_line = line_idx + 1;
    let target_col = col_idx + 1;

    for node in &analysis.ast {
        if let Tree::Fn {
            name: None,
            args,
            body,
        } = &node.tree
        {
            if node.loc.y == target_line && node.loc.x <= target_col {
                let params = analysis.extract_param_names(args);
                let ret = analysis.infer_body_return_type(body);
                let content = format!("```iok\nfn({}) -> {}\n```", params.join(", "), ret);
                return Some(content);
            }
        }
    }
    None
}

fn get_word_at_position(line: &str, col: usize) -> Option<String> {
    if col >= line.len() && !line.is_empty() {
        return get_word_at_position(line, line.len() - 1);
    }
    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() {
        return None;
    }

    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_';
    if !is_ident_char(chars[col]) {
        return None;
    }

    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = col;
    while end + 1 < chars.len() && is_ident_char(chars[end + 1]) {
        end += 1;
    }

    Some(chars[start..=end].iter().collect())
}
