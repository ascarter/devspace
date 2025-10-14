use anstyle::{AnsiColor, Style};
use atty::Stream;
use std::fmt::Display;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const STATUS_WIDTH: usize = 12;

#[derive(Debug, Clone, Copy)]
enum StatusKind {
    Pending,
    Success,
    Info,
    Warn,
    Error,
}

fn supports_color(stream: Stream) -> bool {
    atty::is(stream) && std::env::var_os("NO_COLOR").is_none()
}

fn style_for(kind: StatusKind) -> Style {
    let style = Style::new().bold();
    match kind {
        StatusKind::Pending => style.fg_color(Some(AnsiColor::Cyan.into())),
        StatusKind::Success => style.fg_color(Some(AnsiColor::Green.into())),
        StatusKind::Info => style.fg_color(Some(AnsiColor::Blue.into())),
        StatusKind::Warn => style.fg_color(Some(AnsiColor::Yellow.into())),
        StatusKind::Error => style.fg_color(Some(AnsiColor::Red.into())),
    }
}

fn write_status(kind: StatusKind, label: &str, message: &str) {
    let stream = match kind {
        StatusKind::Warn | StatusKind::Error => Stream::Stderr,
        _ => Stream::Stdout,
    };

    let use_color = supports_color(stream);
    let mut handle: Box<dyn Write> = match stream {
        Stream::Stdout => Box::new(io::stdout().lock()),
        Stream::Stderr => Box::new(io::stderr().lock()),
        Stream::Stdin => unreachable!("stdin stream is not used for status output"),
    };

    let padded_label = if label.is_empty() {
        " ".repeat(STATUS_WIDTH)
    } else {
        format!("{:>width$}", label, width = STATUS_WIDTH)
    };

    let (prefix, suffix) = if use_color {
        let style = style_for(kind);
        (style.render().to_string(), style.render_reset().to_string())
    } else {
        (String::new(), String::new())
    };

    let lines: Vec<&str> = message.split('\n').collect();
    for (idx, line) in lines.iter().enumerate() {
        if idx == 0 {
            let _ = writeln!(handle, "{prefix}{padded_label}{suffix} {line}");
        } else {
            let _ = writeln!(handle, "{:>width$} {line}", "", width = STATUS_WIDTH);
        }
    }
    let _ = handle.flush();
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() >= 60 {
        let minutes = duration.as_secs() / 60;
        let seconds = duration.as_secs() % 60;
        if seconds == 0 {
            format!("{minutes}m")
        } else {
            format!("{minutes}m {seconds}s")
        }
    } else if duration.as_secs_f64() >= 1.0 {
        format!("{:.2}s", duration.as_secs_f64())
    } else if duration.as_millis() >= 1 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{}µs", duration.as_micros())
    }
}

pub fn status(label: &str, message: impl Display) {
    write_status(StatusKind::Pending, label, &message.to_string());
}

pub fn info(message: impl Display) {
    write_status(StatusKind::Info, "Info", &message.to_string());
}

pub fn warn(message: impl Display) {
    write_status(StatusKind::Warn, "Warning", &message.to_string());
}

pub fn error(message: impl Display) {
    write_status(StatusKind::Error, "Error", &message.to_string());
}

pub fn success(label: &str, message: impl Display) {
    write_status(StatusKind::Success, label, &message.to_string());
}

pub struct Progress {
    message: String,
    started: Instant,
    complete: bool,
}

impl Progress {
    pub fn new(label: impl Into<String>, message: impl Into<String>) -> Self {
        let label = label.into();
        let message = message.into();
        write_status(StatusKind::Pending, &label, &message);

        Self {
            message,
            started: Instant::now(),
            complete: false,
        }
    }

    pub fn success(mut self, label: &str, detail: Option<String>) {
        if self.complete {
            return;
        }

        self.complete = true;
        let mut combined = self.message.clone();
        if let Some(detail) = detail {
            if !detail.is_empty() {
                combined.push_str(" ");
                combined.push_str(&detail);
            }
        }
        let elapsed = format_duration(self.started.elapsed());
        combined.push_str(" in ");
        combined.push_str(&elapsed);

        write_status(StatusKind::Success, label, &combined);
    }

    pub fn fail(mut self, label: &str, error: impl Display) {
        if self.complete {
            return;
        }

        self.complete = true;
        let elapsed = format_duration(self.started.elapsed());
        let combined = format!("{} after {}: {}", self.message, elapsed, error);
        write_status(StatusKind::Error, label, &combined);
    }

    pub fn cancel(mut self, reason: impl Display) {
        if self.complete {
            return;
        }

        self.complete = true;
        let combined = format!("{} ({})", self.message, reason);
        write_status(StatusKind::Warn, "Cancelled", &combined);
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if !self.complete {
            let combined = format!("{} (aborted)", self.message);
            write_status(StatusKind::Warn, "Cancelled", &combined);
            self.complete = true;
        }
    }
}
