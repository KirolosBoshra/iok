use crate::analysis::{loc_to_range, DocumentAnalysis};
use lsp_types::{GotoDefinitionResponse, Location, Position, Url};

pub fn get_definition(
    analysis: &DocumentAnalysis,
    uri: &Url,
    position: Position,
) -> Option<GotoDefinitionResponse> {
    let line_idx = position.line as usize;
    let col_idx = position.character as usize;

    let line = analysis.source.lines().nth(line_idx)?;
    let word = get_word_at_position(line, col_idx)?;

    if let Some(sym) = analysis.vars.get(&word) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: loc_to_range(sym.loc),
        }));
    }

    if let Some(func) = analysis.functions.get(&word) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: loc_to_range(func.loc),
        }));
    }

    if let Some(st) = analysis.structs.get(&word) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: loc_to_range(st.loc),
        }));
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
