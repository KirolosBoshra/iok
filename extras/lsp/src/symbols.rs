use crate::analysis::{loc_to_range, DocumentAnalysis};
use lsp_types::{DocumentSymbol, DocumentSymbolResponse, SymbolKind};

pub fn get_document_symbols(analysis: &DocumentAnalysis) -> Option<DocumentSymbolResponse> {
    let mut symbols = Vec::new();

    for (name, st) in &analysis.structs {
        if name == "File" || name == "Socket" || name == "Server" {
            continue;
        }
        let range = loc_to_range(st.loc);
        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: name.clone(),
            detail: Some("struct".to_string()),
            kind: SymbolKind::STRUCT,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        });
    }

    for (name, func) in &analysis.functions {
        let range = loc_to_range(func.loc);
        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: name.clone(),
            detail: Some(format!("fn({})", func.params.join(", "))),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        });
    }

    for (name, var) in &analysis.vars {
        let range = loc_to_range(var.loc);
        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: name.clone(),
            detail: Some(format!("{}", var.ty)),
            kind: SymbolKind::VARIABLE,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        });
    }

    Some(DocumentSymbolResponse::Nested(symbols))
}
