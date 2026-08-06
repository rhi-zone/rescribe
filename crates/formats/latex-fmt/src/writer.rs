//! `Writer<W: Write>` — streaming writer: emit bytes as events arrive, no
//! intermediate buffer.
//!
//! Operates on this crate's [`crate::events::Event`] stream, which is
//! already the tokenizer-level structural shape (no AST-level argument
//! grouping — a `ControlSequence` event is just the name; any `{...}`
//! groups that structurally followed it in the source arrive as their own
//! `GroupOpen`/`GroupClose` events, exactly as `events()` produced them).
//! So each event maps to its literal textual form directly, with no
//! resolution/grouping logic needed here — mirroring how `events()` itself
//! needed no resolution logic to produce them.

use crate::events::Event;
use std::io::Write as IoWrite;

pub struct Writer<W: IoWrite> {
    sink: W,
}

impl<W: IoWrite> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink }
    }

    pub fn write_event(&mut self, event: Event<'_>) {
        match event {
            Event::StartDocument | Event::EndDocument => {}
            Event::Text(s) => {
                let _ = self.sink.write_all(s.as_bytes());
            }
            Event::Comment(s) => {
                let _ = writeln!(self.sink, "%{s}");
            }
            Event::ControlSequence(name) => {
                let _ = write!(self.sink, "\\{name}");
                if name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                    let _ = self.sink.write_all(b" ");
                }
            }
            Event::GroupOpen => {
                let _ = self.sink.write_all(b"{");
            }
            Event::GroupClose => {
                let _ = self.sink.write_all(b"}");
            }
            Event::MathInline(s) => {
                let _ = write!(self.sink, "${s}$");
            }
            Event::MathDisplay(s) => {
                let _ = write!(self.sink, "$${s}$$");
            }
            Event::AlignTab => {
                let _ = self.sink.write_all(b"&");
            }
            Event::Verb {
                star,
                delim,
                content,
            } => {
                let _ = write!(
                    self.sink,
                    "\\verb{}{delim}{content}{delim}",
                    if star { "*" } else { "" }
                );
            }
            Event::VerbatimEnvBody(s) => {
                let _ = self.sink.write_all(s.as_bytes());
            }
            Event::EnvironmentBegin(name) => {
                let _ = write!(self.sink, "\\begin{{{name}}}");
            }
            Event::EnvironmentEnd(name) => {
                let _ = write!(self.sink, "\\end{{{name}}}");
            }
        }
    }

    pub fn finish(self) -> W {
        self.sink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::events_str;

    #[test]
    fn writer_reproduces_source_shape() {
        let src = "\\textbf{hi} $x$ \\begin{itemize}\\item a\\end{itemize}";
        let mut w = Writer::new(Vec::new());
        for ev in events_str(src) {
            w.write_event(ev);
        }
        let out = String::from_utf8(w.finish()).unwrap();
        // Re-tokenizing the output should produce the same event sequence
        // (ignoring Start/EndDocument bookkeeping, which is stable).
        let evs1: Vec<_> = events_str(src).map(|e| e.into_owned()).collect();
        let evs2: Vec<_> = events_str(&out).map(|e| e.into_owned()).collect();
        assert_eq!(
            evs1, evs2,
            "writer output {out:?} did not round-trip to the same events"
        );
    }
}
