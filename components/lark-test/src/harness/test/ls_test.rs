use crate::harness::test::TestContext;
use languageserver_types::{
    ClientCapabilities, DidOpenTextDocumentParams, Hover, HoverContents, InitializeParams,
    InitializeResult, MarkedString, Position, PublishDiagnosticsParams, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams,
};
use lark_language_server::{JsonRPCNotification, JsonRPCResponse, LSPCommand};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::panic;
use std::process::{Command, Stdio};

impl TestContext<'_> {
    crate fn test_language_server(&self) -> Result<(), Box<std::error::Error>> {
        if self.options.expected_hovers.is_empty() {
            return Ok(());
        }

        let mut child_session = ChildSession::spawn();

        // Child that we are initialized
        child_session.send_init(101)?;

        let result = child_session.receive::<JsonRPCResponse<InitializeResult>>()?;

        assert_eq!(result.id, 101);

        // Open the document
        let test_path: &str = self.test_path.to_str().unwrap();
        child_session.send_open(test_path)?;

        let result = child_session.receive::<JsonRPCNotification<PublishDiagnosticsParams>>()?;

        assert_eq!(result.method, "textDocument/publishDiagnostics");
        assert_eq!(result.params.diagnostics.len(), 0);

        // Hover to get the type
        for hover in &self.options.expected_hovers {
            child_session.send_hover(900, test_path, hover.line_num, hover.character_num)?;

            let result = child_session.receive::<JsonRPCResponse<Hover>>()?;
            assert_eq!(result.id, 900);
            match result.result.contents {
                HoverContents::Scalar(MarkedString::String(s)) => {
                    if !hover.message.is_match(&s) {
                        eprintln!(
                            "{}:{}:{}: unexpected hover message: `{}`",
                            self.test_path.display(),
                            hover.line_num + 1,
                            hover.character_num + 1,
                            s,
                        );

                        panic!("unexpected hover: {}", s);
                    }
                }
                x => panic!("Unexpected string type: {:?}", x),
            }
        }

        Ok(())
    }
}

struct ChildSession {
    child: std::process::Child,
}

impl Drop for ChildSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

impl ChildSession {
    fn spawn() -> ChildSession {
        // LS server hardcodes stdin/stdout, so we need a distinct
        // process; use `cargo run ide` to get one.
        let child = Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg("ide")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        ChildSession { child }
    }
    /// Helper function to do the work of sending a result back to the IDE
    fn send<T: Serialize>(&mut self, msg: T) -> Result<(), Box<std::error::Error>> {
        let child_stdin = self.child.stdin.as_mut().ok_or(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "can connect to child stdin",
        ))?;

        let msg_raw = serde_json::to_string(&msg)?;

        child_stdin
            .write_all(format!("Content-Length: {}\r\n\r\n", msg_raw.len()).as_bytes())
            .expect("Failed to write to stdin");
        child_stdin
            .write_all(msg_raw.as_bytes())
            .expect("Failed to write to stdin");
        //let _ = io::stdout().flush();

        Ok(())
    }

    fn receive<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, Box<std::error::Error>> {
        let child_stdout = self.child.stdout.as_mut().ok_or(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "can connect to child stdout",
        ))?;

        let mut buffer = [0; 16];
        child_stdout.read(&mut buffer[..])?;

        let mut digits = String::new();
        let mut digit = [0; 1];
        loop {
            child_stdout.read(&mut digit[..])?;
            let char_digit = digit[0] as char;

            if char_digit.is_digit(10) {
                digits.push(char_digit);
            } else {
                let mut whitespace = [0; 3];
                child_stdout.read(&mut whitespace[..])?;
                break;
            }
        }
        let num_bytes: usize = digits.trim().parse()?;
        let mut buffer = vec![0u8; num_bytes];
        let _ = child_stdout.read_exact(&mut buffer);

        let buffer_string = String::from_utf8(buffer)?;

        let response: T = serde_json::from_str(&buffer_string)?;
        Ok(response)
    }

    fn send_init(&mut self, id: usize) -> Result<(), Box<std::error::Error>> {
        self.send(LSPCommand::initialize {
            id,
            params: InitializeParams {
                process_id: None,
                root_path: None,
                root_uri: None,
                initialization_options: None,
                capabilities: ClientCapabilities {
                    experimental: None,
                    text_document: None,
                    workspace: None,
                },
                trace: None,
                workspace_folders: None,
            },
        })
    }

    fn send_open(&mut self, filepath: &str) -> Result<(), Box<std::error::Error>> {
        let contents = std::fs::read_to_string(filepath)?;
        let path = std::path::Path::new(filepath).canonicalize()?;
        self.send(LSPCommand::didOpen {
            params: DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: url::Url::parse(&format!(
                        "file:///{}",
                        path.to_str().ok_or(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Bad filepath"
                        ))?
                    ))?,
                    language_id: "lark".into(),
                    version: 1,
                    text: contents,
                },
            },
        })
    }

    fn send_hover(
        &mut self,
        id: usize,
        filepath: &str,
        line: u64,
        character: u64,
    ) -> Result<(), Box<std::error::Error>> {
        let path = std::path::Path::new(filepath).canonicalize()?;
        self.send(LSPCommand::hover {
            id,
            params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: url::Url::parse(&format!(
                        "file:///{}",
                        path.to_str().ok_or(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Bad filepath"
                        ))?
                    ))?,
                },
                position: Position { line, character },
            },
        })
    }
}
