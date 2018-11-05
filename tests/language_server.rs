#[cfg(test)]
mod tests {
    use languageserver_types::{
        ClientCapabilities, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        PublishDiagnosticsParams, TextDocumentItem,
    };
    use lark_language_server::{JsonRPCNotification, JsonRPCResponse, LSPCommand};
    use serde::{Deserialize, Serialize};
    use std::io::{Read, Write};
    use std::panic;
    use std::process::{Child, Command, Stdio};

    /// Helper function to do the work of sending a result back to the IDE
    fn send<T: Serialize>(
        msg: T,
        child: &mut std::process::ChildStdin,
    ) -> Result<(), Box<std::error::Error>> {
        let msg_raw = serde_json::to_string(&msg)?;

        child
            .write_all(format!("Content-Length: {}\r\n\r\n", msg_raw.len()).as_bytes())
            .expect("Failed to write to stdin");
        child
            .write_all(msg_raw.as_bytes())
            .expect("Failed to write to stdin");
        //let _ = io::stdout().flush();

        Ok(())
    }

    fn receive<T: for<'de> Deserialize<'de>>(
        child: &mut std::process::ChildStdout,
    ) -> Result<T, Box<std::error::Error>> {
        let mut buffer = [0; 16];
        child.read(&mut buffer[..])?;

        let mut digits = String::new();
        let mut digit = [0; 1];
        loop {
            child.read(&mut digit[..])?;
            let char_digit = digit[0] as char;

            if char_digit.is_digit(10) {
                digits.push(char_digit);
            } else {
                let mut whitespace = [0; 3];
                child.read(&mut whitespace[..])?;
                break;
            }
        }

        let num_bytes: usize = digits.trim().parse()?;
        let mut buffer = vec![0u8; num_bytes];
        let _ = child.read_exact(&mut buffer);

        let buffer_string = String::from_utf8(buffer)?;

        let response: T = serde_json::from_str(&buffer_string)?;
        Ok(response)
    }

    fn send_init(
        stdin: &mut std::process::ChildStdin,
        id: usize,
    ) -> Result<(), Box<std::error::Error>> {
        send(
            LSPCommand::initialize {
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
            },
            stdin,
        )
    }

    fn send_open(
        stdin: &mut std::process::ChildStdin,
        filepath: &str,
    ) -> Result<(), Box<std::error::Error>> {
        let contents = std::fs::read_to_string(filepath)?;
        let path = std::path::Path::new(filepath).canonicalize()?;
        send(
            LSPCommand::didOpen {
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
            },
            stdin,
        )
    }

    fn kill_and_panic(mut child: Child, msg: &'static str) -> ! {
        let _ = child.kill();
        panic!(msg);
    }

    #[test]
    fn find_expected_error_message() {
        let mut child = Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg("ide")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        let child_stdout = match child.stdout.as_mut() {
            Some(s) => s,
            None => kill_and_panic(child, "Failed to open stdout"),
        };
        let child_stdin = match child.stdin.as_mut() {
            Some(s) => s,
            None => {
                kill_and_panic(child, "Failed to open stdin");
            }
        };

        // Child that we are initialized
        if let Err(_) = send_init(child_stdin, 100) {
            kill_and_panic(child, "Could not send init command");
        }
        if let Ok(result) = receive::<JsonRPCResponse<InitializeResult>>(child_stdout) {
            assert_eq!(result.id, 100);
        } else {
            kill_and_panic(child, "Cannot convert to InitializeResult");
        }

        // Open the document
        if let Err(_) = send_open(child_stdin, "samples/minimal.lark") {
            kill_and_panic(child, "Could not send open command");
        }
        if let Ok(result) = receive::<JsonRPCNotification<PublishDiagnosticsParams>>(child_stdout) {
            assert_eq!(result.method, "textDocument/publishDiagnostics");
            assert_eq!(result.params.diagnostics.len(), 1);
            assert_eq!(result.params.diagnostics[0].message, "Mismatched types");
        } else {
            kill_and_panic(child, "Cannot convert to PublishDiagnosticsParams");
        }

        let _ = child.kill();

        //let output = child.wait_with_output().expect("Failed to read stdout");
        //eprintln!("output: {}", String::from_utf8_lossy(&output.stdout));
        //assert_eq!(String::from_utf8_lossy(&output.stdout), "!dlrow ,olleH\n");
    }
}
