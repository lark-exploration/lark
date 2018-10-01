use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use std::io;
use std::io::prelude::{Read, Write};

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
        id: usize,
        params: languageserver_types::DidOpenTextDocumentParams,
    },
    #[serde(rename = "textDocument/didChange")]
    didChange {
        id: usize,
        params: languageserver_types::DidChangeTextDocumentParams,
    },
    #[serde(rename = "textDocument/hover")]
    hover {
        id: usize,
        params: languageserver_types::TextDocumentPositionParams,
    },
    #[serde(rename = "$/cancelRequest")]
    cancelRequest {
        params: languageserver_types::CancelParams,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct LSPResponse<T> {
    jsonrpc: String,
    id: usize,
    result: T,
}
impl<T> LSPResponse<T> {
    pub fn new(id: usize, result: T) -> LSPResponse<T> {
        LSPResponse {
            jsonrpc: "2.0".into(),
            id,
            result,
        }
    }
}

fn send_result<T: Serialize>(id: usize, result: T) {
    let response = LSPResponse::new(id, result);
    let response_raw = serde_json::to_string(&response).unwrap();

    print!("Content-Length: {}\r\n\r\n", response_raw.len());
    print!("{}", response_raw);
    let _ = io::stdout().flush();
}

pub fn lsp_serve() {
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
                    eprintln!("command: {}", buffer_string);

                    let command = serde_json::from_str::<LSPCommand>(&buffer_string);

                    match command {
                        Ok(LSPCommand::initialize { id, .. }) => {
                            let result = languageserver_types::InitializeResult {
                                capabilities: languageserver_types::ServerCapabilities {
                                    text_document_sync: Some(
                                        languageserver_types::TextDocumentSyncCapability::Kind(
                                            languageserver_types::TextDocumentSyncKind::Incremental,
                                        ),
                                    ),
                                    hover_provider: Some(true),
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

                            send_result(id, result);
                        }
                        Ok(LSPCommand::initialized) => {
                            eprintln!("Initialized received");
                        }
                        Ok(LSPCommand::didOpen { id, params }) => {
                            eprintln!("didOpen: id={} {:#?}", id, params);
                        }
                        Ok(LSPCommand::didChange { id, params }) => {
                            eprintln!("didChange: id={} {:#?}", id, params);
                        }
                        Ok(LSPCommand::hover { id, params }) => {
                            eprintln!("hover: id={} {:#?}", id, params);
                            let result = languageserver_types::Hover {
                                contents: languageserver_types::HoverContents::Scalar(
                                    languageserver_types::MarkedString::from_markdown(format!(
                                        "This *is* a hover at {:?}",
                                        params.position
                                    )),
                                ),
                                range: None,
                            };

                            send_result(id, result);
                        }
                        Ok(LSPCommand::cancelRequest { params }) => {
                            eprintln!("cancel request: {:#?}", params);
                        }
                        Err(e) => eprintln!("Error handling command: {:?}", e),
                    }

                    //eprintln!("Command: {:#?}", command);
                }
            }
            Err(error) => eprintln!("error: {}", error),
        }
    }
}
