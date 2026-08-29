mod analysis;
mod completion;
mod definition;
mod diagnostics;
mod hover;
mod symbols;
mod types;

use analysis::DocumentAnalysis;
use completion::get_completions;
use definition::get_definition;
use diagnostics::check_diagnostics;
use hover::get_hover;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, PublishDiagnostics, Notification as _},
    request::{Completion, GotoDefinition, HoverRequest, DocumentSymbolRequest, Request as _},
    CompletionOptions, InitializeParams, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use symbols::get_document_symbols;
use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    iok::logger::Logger::set_lsp_mode(true);
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            all_commit_characters: None,
            work_done_progress_options: Default::default(),
            completion_item: None,
        }),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })?;

    let initialization_params = connection.initialize(server_capabilities)?;
    let _params: InitializeParams = serde_json::from_value(initialization_params)?;

    main_loop(connection)?;
    io_threads.join()?;

    Ok(())
}

fn main_loop(connection: Connection) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut documents: HashMap<Url, DocumentAnalysis> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                match req.method.as_str() {
                    Completion::METHOD => {
                        let (id, params) = cast_request::<Completion>(req)?;
                        let uri = params.text_document_position.text_document.uri;
                        let pos = params.text_document_position.position;

                        let result = if let Some(analysis) = documents.get(&uri) {
                            get_completions(analysis, pos)
                        } else {
                            None
                        };

                        let resp = Response::new_ok(id, result);
                        connection.sender.send(Message::Response(resp))?;
                    }
                    HoverRequest::METHOD => {
                        let (id, params) = cast_request::<HoverRequest>(req)?;
                        let uri = params.text_document_position_params.text_document.uri;
                        let pos = params.text_document_position_params.position;

                        let result = if let Some(analysis) = documents.get(&uri) {
                            get_hover(analysis, pos)
                        } else {
                            None
                        };

                        let resp = Response::new_ok(id, result);
                        connection.sender.send(Message::Response(resp))?;
                    }
                    GotoDefinition::METHOD => {
                        let (id, params) = cast_request::<GotoDefinition>(req)?;
                        let uri = params.text_document_position_params.text_document.uri;
                        let pos = params.text_document_position_params.position;

                        let result = if let Some(analysis) = documents.get(&uri) {
                            get_definition(analysis, &uri, pos)
                        } else {
                            None
                        };

                        let resp = Response::new_ok(id, result);
                        connection.sender.send(Message::Response(resp))?;
                    }
                    DocumentSymbolRequest::METHOD => {
                        let (id, params) = cast_request::<DocumentSymbolRequest>(req)?;
                        let uri = params.text_document.uri;

                        let result = if let Some(analysis) = documents.get(&uri) {
                            get_document_symbols(analysis)
                        } else {
                            None
                        };

                        let resp = Response::new_ok(id, result);
                        connection.sender.send(Message::Response(resp))?;
                    }
                    _ => {}
                }
            }
            Message::Notification(notif) => match notif.method.as_str() {
                DidOpenTextDocument::METHOD => {
                    let params = cast_notification::<DidOpenTextDocument>(notif)?;
                    let uri = params.text_document.uri;
                    let text = params.text_document.text;

                    let analysis = DocumentAnalysis::new(&text);
                    let diags = check_diagnostics(&text);

                    publish_diagnostics(&connection, uri.clone(), diags)?;
                    documents.insert(uri, analysis);
                }
                DidChangeTextDocument::METHOD => {
                    let params = cast_notification::<DidChangeTextDocument>(notif)?;
                    let uri = params.text_document.uri;
                    if let Some(change) = params.content_changes.into_iter().next() {
                        let text = change.text;
                        let analysis = DocumentAnalysis::new(&text);
                        let diags = check_diagnostics(&text);

                        publish_diagnostics(&connection, uri.clone(), diags)?;
                        documents.insert(uri, analysis);
                    }
                }
                _ => {}
            },
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    uri: Url,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params = lsp_types::PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    let notif = Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        params,
    );
    connection.sender.send(Message::Notification(notif))?;
    Ok(())
}

fn cast_request<R>(req: Request) -> Result<(RequestId, R::Params), Box<dyn Error + Sync + Send>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD).map_err(|e| e.into())
}

fn cast_notification<N>(notif: Notification) -> Result<N::Params, Box<dyn Error + Sync + Send>>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    notif.extract(N::METHOD).map_err(|e| e.into())
}
