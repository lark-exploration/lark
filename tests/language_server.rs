#[cfg(test)]
mod tests {
    use languageserver_types::{
        ClientCapabilities, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        PublishDiagnosticsParams, TextDocumentItem,
    };
    use lark_language_server::{JsonRPCNotification, JsonRPCResponse, LSPCommand};
    use serde::{Deserialize, Serialize};
    use std::io::{self, Read, Write};
    use std::process::{Command, Stdio};

    /// Helper function to do the work of sending a result back to the IDE
    fn send<T: Serialize>(msg: T, child: &mut std::process::ChildStdin) {
        let msg_raw = serde_json::to_string(&msg).unwrap();

        child
            .write_all(format!("Content-Length: {}\r\n\r\n", msg_raw.len()).as_bytes())
            .expect("Failed to write to stdin");
        child
            .write_all(msg_raw.as_bytes())
            .expect("Failed to write to stdin");
        //let _ = io::stdout().flush();
    }

    fn receive<T: for<'de> Deserialize<'de>>(
        child: &mut std::process::ChildStdout,
    ) -> Result<T, Box<std::error::Error>> {
        let mut buffer = [0; 25];
        child
            .read(&mut buffer[..])
            .expect("Can not read output from child");
        let input = String::from_utf8_lossy(&buffer);
        let content_length_items: Vec<&str> = input.split(' ').collect();
        if content_length_items[0] == "Content-Length:" {
            let num_bytes: usize = content_length_items[1]
                .replace("\0", "")
                .trim()
                .parse()
                .unwrap();
            let mut buffer = vec![0u8; num_bytes];
            let _ = child.read_exact(&mut buffer);

            let buffer_string = String::from_utf8(buffer).unwrap();

            let response: T = serde_json::from_str(&buffer_string)?;
            Ok(response)
        } else {
            Err(Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing 'Content-Length'",
            )))
        }
    }

    fn send_init(stdin: &mut std::process::ChildStdin, id: usize) {
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
        );
    }

    fn send_open(stdin: &mut std::process::ChildStdin, filepath: &str) {
        let contents = std::fs::read_to_string(filepath).unwrap();
        let path = std::path::Path::new(filepath).canonicalize().unwrap();
        send(
            LSPCommand::didOpen {
                params: DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: url::Url::parse(&format!("file:///{}", path.to_str().unwrap()))
                            .unwrap(),
                        language_id: "lark".into(),
                        version: 1,
                        text: contents,
                    },
                },
            },
            stdin,
        );
    }

    #[test]
    fn init() {
        let mut child = Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg("ide")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        let child_stdout = child.stdout.as_mut().expect("Failed to open stdout");
        let child_stdin = child.stdin.as_mut().expect("Failed to open stdin");

        // Child that we are initialized
        send_init(child_stdin, 100);
        let result = receive::<JsonRPCResponse<InitializeResult>>(child_stdout).unwrap();
        assert_eq!(result.id, 100);

        // Open the document
        send_open(child_stdin, "samples/minimal.lark");
        let result =
            receive::<JsonRPCNotification<PublishDiagnosticsParams>>(child_stdout).unwrap();
        assert_eq!(result.method, "textDocument/publishDiagnostics");

        let _ = child.kill();

        //let output = child.wait_with_output().expect("Failed to read stdout");
        //eprintln!("output: {}", String::from_utf8_lossy(&output.stdout));
        //assert_eq!(String::from_utf8_lossy(&output.stdout), "!dlrow ,olleH\n");
    }
}
