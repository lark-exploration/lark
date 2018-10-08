use crate::task_manager::{self, Actor, LspRequest, LspResponse, MsgToManager};
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use std::io;
use std::io::prelude::{Read, Write};
use std::sync::mpsc::Sender;

pub use languageserver_types::Position;

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
struct LSPJsonRPC<T> {
    jsonrpc: String,
    id: usize,
    result: T,
}
impl<T> LSPJsonRPC<T> {
    pub fn new(id: usize, result: T) -> LSPJsonRPC<T> {
        LSPJsonRPC {
            jsonrpc: "2.0".into(),
            id,
            result,
        }
    }
}

fn send_result<T: Serialize>(id: usize, result: T) {
    let response = LSPJsonRPC::new(id, result);
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

    fn startup(&mut self, _: Box<dyn Fn(Self::OutMessage) -> () + Send>) {}

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

                send_result(id, result);
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

                send_result(id, result);
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
                        completion_provider: Some(languageserver_types::CompletionOptions {
                            resolve_provider: Some(true),
                            trigger_characters: Some(vec![".".into()]),
                        }),
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

                send_result(id, result);
            }
        }
    }
}

pub fn lsp_serve(send_to_manager_channel: Sender<task_manager::MsgToManager>) {
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
                        }
                        Ok(LSPCommand::didChange { params }) => {
                            eprintln!("didChange: {:#?}", params);
                        }
                        Ok(LSPCommand::hover { id, params }) => {
                            eprintln!("hover: id={} {:#?}", id, params);

                            //FIXME: this is using a fake position
                            let _ = send_to_manager_channel.send(MsgToManager::LspRequest(
                                LspRequest::TypeForPos(id, params.position.clone()),
                            ));
                        }
                        Ok(LSPCommand::completion { id, params }) => {
                            eprintln!("completion: id={} {:#?}", id, params);

                            //FIXME: this is using a fake position
                            let _ = send_to_manager_channel.send(MsgToManager::LspRequest(
                                LspRequest::Completion(id, params.position.clone()),
                            ));
                        }
                        Ok(LSPCommand::completionItemResolve { id, params }) => {
                            //Note: this is here in case we need it, though it looks like it's only used
                            //for more expensive computations on a completion (like fetching the docs)
                            eprintln!("resolve completion item: id={} {:#?}", id, params);
                        }
                        Ok(LSPCommand::cancelRequest { params }) => {
                            eprintln!("cancel request: {:#?}", params);
                        }
                        Err(e) => eprintln!("Error handling command: {:?}", e),
                    }
                }
            }
            Err(error) => eprintln!("error: {}", error),
        }
    }
}
