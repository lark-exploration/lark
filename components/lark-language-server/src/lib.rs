use lark_task_manager::{self, Actor, LspRequest, LspResponse, MsgToManager, SendChannel};
use serde::Serialize;
use serde_derive::{Deserialize, Serialize};
use std::io;
use std::io::prelude::{Read, Write};
use std::sync::mpsc::Sender;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
enum LSPCommand {
    initialize {
        id: usize,
        params: languageserver_types::InitializeParams,
    },
    initialized,
    #[serde(rename = "textDocument/didOpen")]
    didOpen {
        params: languageserver_types::DidOpenTextDocumentParams,
    },
    #[serde(rename = "textDocument/didChange")]
    didChange {
        params: languageserver_types::DidChangeTextDocumentParams,
    },
    #[serde(rename = "textDocument/hover")]
    hover {
        id: usize,
        params: languageserver_types::TextDocumentPositionParams,
    },
    #[serde(rename = "textDocument/completion")]
    completion {
        id: usize,
        params: languageserver_types::CompletionParams,
    },
    #[serde(rename = "$/cancelRequest")]
    cancelRequest {
        params: languageserver_types::CancelParams,
    },
    #[serde(rename = "completionItem/resolve")]
    completionItemResolve {
        id: usize,
        params: languageserver_types::CompletionItem,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRPCResponse<T> {
    jsonrpc: String,
    id: usize,
    result: T,
}
impl<T> JsonRPCResponse<T> {
    pub fn new(id: usize, result: T) -> Self {
        JsonRPCResponse {
            jsonrpc: "2.0".into(),
            id,
            result,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRPCNotification<T> {
    jsonrpc: String,
    method: String,
    params: T,
}
impl<T> JsonRPCNotification<T> {
    pub fn new(method: String, params: T) -> Self {
        JsonRPCNotification {
            jsonrpc: "2.0".into(),
            method,
            params,
        }
    }
}

fn send_response<T: Serialize>(id: usize, result: T) {
    let response = JsonRPCResponse::new(id, result);
    let response_raw = serde_json::to_string(&response).unwrap();

    print!("Content-Length: {}\r\n\r\n", response_raw.len());
    print!("{}", response_raw);
    let _ = io::stdout().flush();
}

fn send_notification<T: Serialize>(method: String, notice: T) {
    let response = JsonRPCNotification::new(method, notice);
    let response_raw = serde_json::to_string(&response).unwrap();

    print!("Content-Length: {}\r\n\r\n", response_raw.len());
    print!("{}", response_raw);
    let _ = io::stdout().flush();
}

/// The LSP service is split into two parts:
///   * The server, which handles incoming requests from the IDE
///   * The responder, which sends out results when they're ready
/// The server sends messages *to* the task manager for work that
/// needs to be done. The responder receives messages *from* the
/// task manager for work that has been accomplished.
pub struct LspResponder;

impl Actor for LspResponder {
    type InMessage = LspResponse;
    type OutMessage = ();

    fn startup(&mut self, _: &dyn SendChannel<Self::OutMessage>) {}

    fn shutdown(&mut self) {}

    fn receive_message(&mut self, message: Self::InMessage) {
        match message {
            LspResponse::Type(id, ty) => {
                let result = languageserver_types::Hover {
                    contents: languageserver_types::HoverContents::Scalar(
                        languageserver_types::MarkedString::from_markdown(ty),
                    ),
                    range: None,
                };

                send_response(id, result);
            }
            LspResponse::Completions(id, completions) => {
                let mut completion_items = vec![];

                for completion in completions {
                    completion_items.push(languageserver_types::CompletionItem::new_simple(
                        completion.0,
                        completion.1,
                    ));
                }

                let result = languageserver_types::CompletionList {
                    is_incomplete: false,
                    items: completion_items,
                };

                send_response(id, result);
            }
            LspResponse::Initialized(id) => {
                let result = languageserver_types::InitializeResult {
                    capabilities: languageserver_types::ServerCapabilities {
                        text_document_sync: Some(
                            languageserver_types::TextDocumentSyncCapability::Kind(
                                languageserver_types::TextDocumentSyncKind::Incremental,
                            ),
                        ),
                        hover_provider: Some(true),
                        /*
                        completion_provider: Some(languageserver_types::CompletionOptions {
                            resolve_provider: Some(true),
                            trigger_characters: Some(vec![".".into()]),
                        }),
                        */
                        completion_provider: None,
                        signature_help_provider: None,
                        definition_provider: None,
                        type_definition_provider: None,
                        implementation_provider: None,
                        references_provider: None,
                        document_highlight_provider: None,
                        document_symbol_provider: None,
                        workspace_symbol_provider: None,
                        code_action_provider: None,
                        code_lens_provider: None,
                        document_formatting_provider: None,
                        document_range_formatting_provider: None,
                        document_on_type_formatting_provider: None,
                        rename_provider: None,
                        color_provider: None,
                        folding_range_provider: None,
                        execute_command_provider: None,
                        workspace: None,
                    },
                };

                send_response(id, result);
            }
            LspResponse::Diagnostics(url, diagnostics) => {
                let lsp_diagnostics: Vec<languageserver_types::Diagnostic> = diagnostics
                    .iter()
                    .map(|(range, diag)| {
                        languageserver_types::Diagnostic::new_simple(*range, diag.clone())
                    })
                    .collect();

                let notice = languageserver_types::PublishDiagnosticsParams {
                    uri: url,
                    diagnostics: lsp_diagnostics,
                };

                send_notification("textDocument/publishDiagnostics".into(), notice);
            }
        }
    }
}

pub fn lsp_serve(send_to_manager_channel: Sender<lark_task_manager::MsgToManager>) {
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let content_length_items: Vec<&str> = input.split(' ').collect();
                if content_length_items[0] == "Content-Length:" {
                    let num_bytes: usize = content_length_items[1].trim().parse().unwrap();
                    let mut buffer = vec![0u8; num_bytes + 2];
                    let _ = io::stdin().read_exact(&mut buffer);

                    let buffer_string = String::from_utf8(buffer).unwrap();

                    let command = serde_json::from_str::<LSPCommand>(&buffer_string);

                    match command {
                        Ok(LSPCommand::initialize { id, .. }) => {
                            let _ = send_to_manager_channel
                                .send(MsgToManager::LspRequest(LspRequest::Initialize(id)));
                        }
                        Ok(LSPCommand::initialized) => {
                            eprintln!("Initialized received");
                        }
                        Ok(LSPCommand::didOpen { params }) => {
                            eprintln!("didOpen: {:#?}", params);

                            let _ = send_to_manager_channel.send(MsgToManager::LspRequest(
                                LspRequest::OpenFile(
                                    params.text_document.uri.clone(),
                                    params.text_document.text.clone(),
                                ),
                            ));
                        }
                        Ok(LSPCommand::didChange { params }) => {
                            eprintln!("didChange: {:#?}", params);
                        }
                        Ok(LSPCommand::hover { id, params }) => {
                            eprintln!("hover: id={} {:#?}", id, params);

                            let _ = send_to_manager_channel.send(MsgToManager::LspRequest(
                                LspRequest::TypeForPos(
                                    id,
                                    params.text_document.uri.clone(),
                                    params.position.clone(),
                                ),
                            ));
                        }
                        Ok(LSPCommand::completion { id, params }) => {
                            eprintln!("completion: id={} {:#?}", id, params);
                        }
                        Ok(LSPCommand::completionItemResolve { id, params }) => {
                            //Note: this is here in case we need it, though it looks like it's only used
                            //for more expensive computations on a completion (like fetching the docs)
                            eprintln!("resolve completion item: id={} {:#?}", id, params);
                        }
                        Ok(LSPCommand::cancelRequest {
                            params: languageserver_types::CancelParams { id },
                        }) => match id {
                            languageserver_types::NumberOrString::Number(num) => {
                                eprintln!("cancelling item: id={}", num);
                                let _ = send_to_manager_channel
                                    .send(MsgToManager::Cancel(num as usize));
                            }
                            _ => unimplemented!(
                                "Non-number cancellation IDs not currently supported"
                            ),
                        },
                        Err(e) => eprintln!("Error handling command: {:?}", e),
                    }
                }
            }
            Err(error) => eprintln!("error: {}", error),
        }
    }
}
