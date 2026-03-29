//! Streaming Fountain writer — converts a stream of events to Fountain text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use fountain_fmt::writer::Writer;
//! use fountain_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartSceneHeading);
//! w.write_event(OwnedEvent::Text("INT. OFFICE - DAY".to_string().into()));
//! w.write_event(OwnedEvent::EndSceneHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::*;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming Fountain writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush Fountain text to the underlying sink and
/// recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            events: Vec::new(),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as Fountain text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = events_to_doc(std::mem::take(&mut self.events));
        let text = crate::emit::emit(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

// ── Event -> AST reconstruction ──────────────────────────────────────────────

fn events_to_doc(events: Vec<OwnedEvent>) -> FountainDoc {
    let mut builder = DocBuilder::new();
    for event in events {
        builder.process(event);
    }
    builder.finish()
}

enum Frame {
    Document {
        metadata: std::collections::BTreeMap<String, String>,
        blocks: Vec<Block>,
    },
    DialogueBlock {
        blocks: Vec<Block>,
    },
    SceneHeading {
        text: String,
    },
    Action {
        text: String,
    },
    Character {
        name: String,
        dual: bool,
    },
    Dialogue {
        text: String,
    },
    Parenthetical {
        text: String,
    },
    Transition {
        text: String,
    },
    Centered {
        text: String,
    },
    Lyric {
        text: String,
    },
    Note {
        text: String,
    },
    Synopsis {
        text: String,
    },
    Section {
        level: usize,
        text: String,
    },
    Boneyard {
        text: String,
    },
}

struct DocBuilder {
    stack: Vec<Frame>,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder {
            stack: vec![Frame::Document {
                metadata: std::collections::BTreeMap::new(),
                blocks: Vec::new(),
            }],
        }
    }

    fn process(&mut self, event: OwnedEvent) {
        match event {
            OwnedEvent::StartDocument => {}
            OwnedEvent::EndDocument => {}

            OwnedEvent::Metadata { key, value } => {
                if let Some(Frame::Document { metadata, .. }) = self.stack.first_mut() {
                    metadata.insert(key.into_owned(), value.into_owned());
                }
            }

            OwnedEvent::StartDialogueBlock => {
                self.stack.push(Frame::DialogueBlock {
                    blocks: Vec::new(),
                });
            }
            OwnedEvent::EndDialogueBlock => {
                if let Some(Frame::DialogueBlock { blocks }) = self.stack.pop() {
                    self.push_blocks(blocks);
                }
            }

            OwnedEvent::StartSceneHeading => {
                self.stack.push(Frame::SceneHeading {
                    text: String::new(),
                });
            }
            OwnedEvent::EndSceneHeading => {
                if let Some(Frame::SceneHeading { text }) = self.stack.pop() {
                    self.push_block(Block::SceneHeading {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartAction => {
                self.stack.push(Frame::Action {
                    text: String::new(),
                });
            }
            OwnedEvent::EndAction => {
                if let Some(Frame::Action { text }) = self.stack.pop() {
                    self.push_block(Block::Action {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartCharacter { dual } => {
                self.stack.push(Frame::Character {
                    name: String::new(),
                    dual,
                });
            }
            OwnedEvent::EndCharacter => {
                if let Some(Frame::Character { name, dual }) = self.stack.pop() {
                    self.push_block(Block::Character {
                        name,
                        dual,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartDialogue => {
                self.stack.push(Frame::Dialogue {
                    text: String::new(),
                });
            }
            OwnedEvent::EndDialogue => {
                if let Some(Frame::Dialogue { text }) = self.stack.pop() {
                    self.push_block(Block::Dialogue {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartParenthetical => {
                self.stack.push(Frame::Parenthetical {
                    text: String::new(),
                });
            }
            OwnedEvent::EndParenthetical => {
                if let Some(Frame::Parenthetical { text }) = self.stack.pop() {
                    self.push_block(Block::Parenthetical {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartTransition => {
                self.stack.push(Frame::Transition {
                    text: String::new(),
                });
            }
            OwnedEvent::EndTransition => {
                if let Some(Frame::Transition { text }) = self.stack.pop() {
                    self.push_block(Block::Transition {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartCentered => {
                self.stack.push(Frame::Centered {
                    text: String::new(),
                });
            }
            OwnedEvent::EndCentered => {
                if let Some(Frame::Centered { text }) = self.stack.pop() {
                    self.push_block(Block::Centered {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartLyric => {
                self.stack.push(Frame::Lyric {
                    text: String::new(),
                });
            }
            OwnedEvent::EndLyric => {
                if let Some(Frame::Lyric { text }) = self.stack.pop() {
                    self.push_block(Block::Lyric {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartNote => {
                self.stack.push(Frame::Note {
                    text: String::new(),
                });
            }
            OwnedEvent::EndNote => {
                if let Some(Frame::Note { text }) = self.stack.pop() {
                    self.push_block(Block::Note {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartSynopsis => {
                self.stack.push(Frame::Synopsis {
                    text: String::new(),
                });
            }
            OwnedEvent::EndSynopsis => {
                if let Some(Frame::Synopsis { text }) = self.stack.pop() {
                    self.push_block(Block::Synopsis {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::StartSection { level } => {
                self.stack.push(Frame::Section {
                    level,
                    text: String::new(),
                });
            }
            OwnedEvent::EndSection => {
                if let Some(Frame::Section { level, text }) = self.stack.pop() {
                    self.push_block(Block::Section {
                        level,
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::PageBreak => {
                self.push_block(Block::PageBreak { span: Span::NONE });
            }

            OwnedEvent::StartBoneyard => {
                self.stack.push(Frame::Boneyard {
                    text: String::new(),
                });
            }
            OwnedEvent::EndBoneyard => {
                if let Some(Frame::Boneyard { text }) = self.stack.pop() {
                    self.push_block(Block::Boneyard {
                        text,
                        span: Span::NONE,
                    });
                }
            }

            OwnedEvent::Text(cow) => {
                self.push_text(&cow);
            }
        }
    }

    fn push_text(&mut self, text: &str) {
        match self.stack.last_mut() {
            Some(Frame::SceneHeading { text: t }) => t.push_str(text),
            Some(Frame::Action { text: t }) => t.push_str(text),
            Some(Frame::Character { name, .. }) => name.push_str(text),
            Some(Frame::Dialogue { text: t }) => t.push_str(text),
            Some(Frame::Parenthetical { text: t }) => t.push_str(text),
            Some(Frame::Transition { text: t }) => t.push_str(text),
            Some(Frame::Centered { text: t }) => t.push_str(text),
            Some(Frame::Lyric { text: t }) => t.push_str(text),
            Some(Frame::Note { text: t }) => t.push_str(text),
            Some(Frame::Synopsis { text: t }) => t.push_str(text),
            Some(Frame::Section { text: t, .. }) => t.push_str(text),
            Some(Frame::Boneyard { text: t }) => t.push_str(text),
            _ => {}
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(Frame::Document { blocks, .. }) => blocks.push(block),
            Some(Frame::DialogueBlock { blocks }) => blocks.push(block),
            _ => {}
        }
    }

    fn push_blocks(&mut self, mut blocks: Vec<Block>) {
        if let Some(Frame::Document {
            blocks: doc_blocks, ..
        }) = self.stack.last_mut()
        {
            doc_blocks.append(&mut blocks);
        }
    }

    fn finish(mut self) -> FountainDoc {
        match self.stack.pop() {
            Some(Frame::Document { metadata, blocks }) => FountainDoc {
                metadata,
                blocks,
                span: Span::NONE,
            },
            _ => FountainDoc::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_scene_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartDocument);
        w.write_event(OwnedEvent::StartSceneHeading);
        w.write_event(OwnedEvent::Text("INT. OFFICE - DAY".to_string().into()));
        w.write_event(OwnedEvent::EndSceneHeading);
        w.write_event(OwnedEvent::EndDocument);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("INT. OFFICE - DAY"), "got: {s:?}");
    }

    #[test]
    fn test_writer_action() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartDocument);
        w.write_event(OwnedEvent::StartAction);
        w.write_event(OwnedEvent::Text("John enters.".to_string().into()));
        w.write_event(OwnedEvent::EndAction);
        w.write_event(OwnedEvent::EndDocument);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("John enters."), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "INT. OFFICE - DAY\n\nJohn sits down.\n\nCUT TO:\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }
}
