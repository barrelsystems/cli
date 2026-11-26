use crate::cli::OutputFormat;
use crate::terminal::Terminal;
use std::time::Duration;

/// Visual preview only: no browser, authentication request, or credential storage.
pub(crate) fn run(terminal: Terminal, output: OutputFormat) {
    if output == OutputFormat::Json {
        println!(
            "{{\"preview\":true,\"authenticated\":false,\"credentialSource\":\"BARREL_CREDENTIAL\"}}"
        );
        return;
    }

    terminal.introduction("Sign in", "Animation preview · no authentication requests");
    if terminal.interactive() {
        terminal.note("Your browser will handle sign-in when authentication is available.\n");
        let _: Result<(), std::convert::Infallible> =
            terminal.task_with_status("Sign-in preview complete", |signal| {
                for label in [
                    "Preparing browser handoff",
                    "Waiting for browser confirmation",
                    "Returning to your terminal",
                ] {
                    signal.set_label(label);
                    // Simulated work is restricted to this explicitly labeled preview.
                    if terminal.motion() {
                        std::thread::sleep(Duration::from_millis(1200));
                    }
                }
                Ok(())
            });
    }
    println!("\nSign-in preview complete. No browser was opened and no session was created.");
    println!("For real commands, provide your application credential through BARREL_CREDENTIAL.");
}
