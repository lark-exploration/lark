#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::panic;
    use std::process::{Command, Stdio};

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
            let child = Command::new("cargo")
                .arg("run")
                .arg("--")
                .arg("repl")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to spawn child process");

            ChildSession { child }
        }
        /// Helper function to do the work of sending a result back to the IDE
        fn send(&mut self, msg: &str) -> Result<(), Box<std::error::Error>> {
            let child_stdin = self.child.stdin.as_mut().ok_or(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "can connect to child stdin",
            ))?;

            child_stdin
                .write_all(msg.as_bytes())
                .expect("Failed to write to stdin");
            //let _ = io::stdout().flush();

            Ok(())
        }

        fn receive(&mut self) -> Result<String, Box<std::error::Error>> {
            let child_stdout = self.child.stdout.as_mut().ok_or(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "can connect to child stdout",
            ))?;

            let mut buffer = String::new();
            let mut character = [0; 1];
            loop {
                child_stdout.read(&mut character[..])?;
                let ch = character[0] as char;

                eprintln!("{}", ch);

                buffer.push(ch);

                if ch == '>' {
                    eprintln!("break");
                    break;
                }
            }

            Ok(buffer)
        }
    }

    #[test]
    fn repl_test_assignment() {
        let mut child_session = ChildSession::spawn();

        let _ = child_session.receive();

        child_session.send("let x = true\n").unwrap();
        let _result = child_session.receive().unwrap();

        child_session.send("debug(x)\n").unwrap();
        let result = child_session.receive().unwrap();

        assert_eq!(result, " true\n>");
    }
}
