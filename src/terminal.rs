use crate::cli::OutputFormat;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const SIGNAL_DELAY: Duration = Duration::from_millis(300);
const SIGNAL_INTERVAL: Duration = Duration::from_millis(80);

const SIGNAL_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"];

#[derive(Clone, Copy)]
pub struct Terminal {
    interactive: bool,
    color: bool,
    motion: bool,
}

pub struct Options {
    pub output: OutputFormat,
    pub no_color: bool,
    pub no_motion: bool,
}

impl Terminal {
    pub fn detect(options: Options) -> Self {
        let ci = std::env::var("CI").is_ok_and(|value| !value.is_empty() && value != "false");
        let dumb = std::env::var("TERM").map_or(true, |term| term == "dumb");
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        let no_motion = std::env::var_os("BARREL_NO_MOTION").is_some_and(|value| !value.is_empty());
        let interactive = options.output == OutputFormat::Text
            && io::stdout().is_terminal()
            && io::stderr().is_terminal()
            && !ci
            && !dumb;

        Self {
            interactive,
            color: interactive && !options.no_color && !no_color,
            motion: interactive && !options.no_motion && !no_motion,
        }
    }

    pub const fn interactive(self) -> bool {
        self.interactive
    }

    pub const fn motion(self) -> bool {
        self.motion
    }

    pub fn introduction(self, section: &str, context: &str) {
        if !self.interactive {
            return;
        }

        line("\n  ▃▃▃▃\n  ▆▆▆▆\n  ▆▆▆▆\n  ▆▆▆▆");
        self.heading(section, context);
    }

    pub fn heading(self, section: &str, context: &str) {
        if !self.interactive {
            return;
        }

        line(&format!("\n  Barrel / {section}"));
        let context: String = context
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        line(&format!("  {context}\n"));
    }

    pub fn note(self, label: &str) {
        if self.interactive {
            line(&format!("  {label}"));
        }
    }

    pub fn task<T, E>(self, label: &str, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        self.task_with_status(label, |_| operation())
    }

    pub fn task_with_status<T, E>(
        self,
        label: &str,
        operation: impl FnOnce(&Signal) -> Result<T, E>,
    ) -> Result<T, E> {
        let signal = Signal::start(self, label);
        let result = operation(&signal);
        drop(signal);

        if self.interactive {
            let (mark, color) = if result.is_ok() {
                ("✓", "32")
            } else {
                ("×", "31")
            };
            let marker = if self.color {
                format!("\x1b[{color}m{mark}\x1b[0m")
            } else {
                mark.to_owned()
            };
            line(&format!("  {marker}  {label}"));
        }

        result
    }
}

// Presentation failure must not change whether an API mutation succeeded.
fn line(text: &str) {
    let _ = writeln!(io::stderr().lock(), "{text}");
}

pub struct Signal {
    stop: Option<Sender<SignalMessage>>,
    worker: Option<JoinHandle<()>>,
}

enum SignalMessage {
    Label(String),
    Stop,
}

impl Signal {
    pub fn set_label(&self, label: &str) {
        if let Some(send) = &self.stop {
            let _ = send.send(SignalMessage::Label(label.to_owned()));
        }
    }

    fn start(terminal: Terminal, label: &str) -> Self {
        let mut signal = Self {
            stop: None,
            worker: None,
        };
        if !terminal.motion {
            return signal;
        }

        let label = label.to_owned();
        let (send, receive) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("terminal-signal".into())
            .spawn(move || {
                // Fast work never paints a transient spinner or waits for its first frame.
                let deadline = std::time::Instant::now() + SIGNAL_DELAY;
                let mut label = label;
                loop {
                    match receive
                        .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    {
                        Ok(SignalMessage::Label(next)) => label = next,
                        Err(RecvTimeoutError::Timeout) => break,
                        _ => return,
                    }
                }

                let Ok(style) = ProgressStyle::with_template("  {spinner}  {msg}") else {
                    return;
                };
                let bar = ProgressBar::new_spinner().with_style(style.tick_strings(SIGNAL_FRAMES));
                bar.set_message(label);
                loop {
                    bar.tick();
                    let deadline = std::time::Instant::now() + SIGNAL_INTERVAL;
                    loop {
                        match receive.recv_timeout(
                            deadline.saturating_duration_since(std::time::Instant::now()),
                        ) {
                            Ok(SignalMessage::Label(next)) => bar.set_message(next),
                            Err(RecvTimeoutError::Timeout) => break,
                            _ => {
                                bar.finish_and_clear();
                                return;
                            }
                        }
                    }
                }
            });

        if let Ok(worker) = worker {
            signal.stop = Some(send);
            signal.worker = Some(worker);
        }
        signal
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(SignalMessage::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_signal_interrupts_the_initial_delay() {
        let start = std::time::Instant::now();
        let terminal = Terminal {
            interactive: true,
            color: true,
            motion: true,
        };
        drop(Signal::start(terminal, "Fast operation"));
        assert!(start.elapsed() < Duration::from_millis(200));
    }
}
