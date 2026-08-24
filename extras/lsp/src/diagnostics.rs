use crate::analysis::loc_to_range;
use iok::lexer::{Lexer, TokenType};
use lsp_types::{Diagnostic, DiagnosticSeverity};

pub fn check_diagnostics(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let source_str = source.to_string();
    let mut lexer = Lexer::new(&source_str);
    let tokens = lexer.tokenize();

    let mut brace_stack = Vec::new();
    let mut paren_stack = Vec::new();
    let mut square_stack = Vec::new();

    for token in tokens {
        match token.token {
            TokenType::OpenCurly => brace_stack.push(token.loc),
            TokenType::CloseCurly => {
                if brace_stack.pop().is_none() {
                    diagnostics.push(Diagnostic {
                        range: loc_to_range(token.loc),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("iok-lsp".to_string()),
                        message: "Unmatched closing brace '}'".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            TokenType::OpenParen => paren_stack.push(token.loc),
            TokenType::CloseParen => {
                if paren_stack.pop().is_none() {
                    diagnostics.push(Diagnostic {
                        range: loc_to_range(token.loc),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("iok-lsp".to_string()),
                        message: "Unmatched closing parenthesis ')'".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            TokenType::OpenSquare => square_stack.push(token.loc),
            TokenType::CloseSquare => {
                if square_stack.pop().is_none() {
                    diagnostics.push(Diagnostic {
                        range: loc_to_range(token.loc),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("iok-lsp".to_string()),
                        message: "Unmatched closing bracket ']'".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            _ => {}
        }
    }

    for loc in brace_stack {
        diagnostics.push(Diagnostic {
            range: loc_to_range(loc),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("iok-lsp".to_string()),
            message: "Unclosed opening brace '{'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    for loc in paren_stack {
        diagnostics.push(Diagnostic {
            range: loc_to_range(loc),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("iok-lsp".to_string()),
            message: "Unclosed opening parenthesis '('".to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    for loc in square_stack {
        diagnostics.push(Diagnostic {
            range: loc_to_range(loc),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("iok-lsp".to_string()),
            message: "Unclosed opening bracket '['".to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    diagnostics
}
