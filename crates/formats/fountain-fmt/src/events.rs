//! Streaming event iterator over a parsed `FountainDoc`.

use std::borrow::Cow;

use crate::ast::{Block, FountainDoc};

/// A streaming event from a Fountain document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Document events ──────────────────────────────────────────────────────
    StartDocument,
    EndDocument,

    // ── Block events ─────────────────────────────────────────────────────────
    StartSceneHeading,
    EndSceneHeading,
    StartAction,
    EndAction,
    StartDialogueBlock,
    EndDialogueBlock,
    StartCharacter {
        dual: bool,
    },
    EndCharacter,
    StartDialogue,
    EndDialogue,
    StartParenthetical,
    EndParenthetical,
    StartTransition,
    EndTransition,
    StartCentered,
    EndCentered,
    StartLyric,
    EndLyric,
    StartNote,
    EndNote,
    StartSynopsis,
    EndSynopsis,
    StartSection {
        level: usize,
    },
    EndSection,
    PageBreak,
    StartBoneyard,
    EndBoneyard,

    // ── Leaf / inline events ─────────────────────────────────────────────────
    Text(Cow<'a, str>),
    Metadata {
        key: Cow<'a, str>,
        value: Cow<'a, str>,
    },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::Metadata { key, value } => Event::Metadata {
                key: Cow::Owned(key.into_owned()),
                value: Cow::Owned(value.into_owned()),
            },
            // Safety: all other variants contain only owned fields or no references.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── Pull iterator ────────────────────────────────────────────────────────────

/// Pull-mode event iterator over a Fountain document.
///
/// The iterator owns the parsed `FountainDoc` and yields events by walking the
/// AST lazily.
pub struct EventIter<'a> {
    doc: &'a FountainDoc,
    /// Phase of iteration: metadata, blocks, done.
    phase: Phase,
    /// Current block index.
    block_idx: usize,
    /// State within the current block.
    inner: InnerState,
    /// Metadata keys, iterated in order.
    meta_keys: Vec<&'a String>,
    meta_idx: usize,
    /// Whether we have emitted StartDocument.
    started: bool,
    /// Whether we have emitted EndDocument.
    finished: bool,
    /// Pending events from dialogue block expansion.
    pending: Vec<Event<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Metadata,
    Blocks,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InnerState {
    /// Ready to emit the start event for the current block.
    Start,
    /// Text content has been emitted; emit the end event next.
    End,
}

impl<'a> EventIter<'a> {
    pub fn new(doc: &'a FountainDoc) -> Self {
        let meta_keys: Vec<&'a String> = doc.metadata.keys().collect();
        EventIter {
            doc,
            phase: Phase::Metadata,
            block_idx: 0,
            inner: InnerState::Start,
            meta_keys,
            meta_idx: 0,
            started: false,
            finished: false,
            pending: Vec::new(),
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        // Drain pending events first (from dialogue block expansion).
        if let Some(ev) = self.pending.pop() {
            return Some(ev);
        }

        if !self.started {
            self.started = true;
            return Some(Event::StartDocument);
        }

        if self.finished {
            return None;
        }

        // ── Metadata phase ──────────────────────────────────────────────────
        if self.phase == Phase::Metadata {
            if self.meta_idx < self.meta_keys.len() {
                let key = self.meta_keys[self.meta_idx];
                let value = &self.doc.metadata[key];
                self.meta_idx += 1;
                return Some(Event::Metadata {
                    key: Cow::Borrowed(key.as_str()),
                    value: Cow::Borrowed(value.as_str()),
                });
            }
            self.phase = Phase::Blocks;
        }

        // ── Blocks phase ────────────────────────────────────────────────────
        if self.phase == Phase::Blocks {
            if self.block_idx >= self.doc.blocks.len() {
                self.phase = Phase::Done;
                self.finished = true;
                return Some(Event::EndDocument);
            }

            let block = &self.doc.blocks[self.block_idx];

            match self.inner {
                InnerState::Start => {
                    // Check for dialogue block pattern: Character followed by
                    // Dialogue/Parenthetical blocks.
                    if matches!(block, Block::Character { .. }) {
                        return Some(self.expand_dialogue_block());
                    }

                    self.inner = InnerState::End;
                    Some(start_event_for(block))
                }
                InnerState::End => {
                    self.inner = InnerState::Start;
                    self.block_idx += 1;
                    Some(end_event_for(block))
                }
            }
        } else {
            self.finished = true;
            Some(Event::EndDocument)
        }
    }
}

impl<'a> EventIter<'a> {
    /// Expand a character + following dialogue/parenthetical blocks into a
    /// dialogue block event sequence. Returns the first event; remaining
    /// events are pushed onto `self.pending` (in reverse order for pop()).
    fn expand_dialogue_block(&mut self) -> Event<'a> {
        let mut events = Vec::new();

        // StartDialogueBlock
        events.push(Event::StartDialogueBlock);

        // Character
        let char_block = &self.doc.blocks[self.block_idx];
        events.push(start_event_for(char_block));
        events.push(text_event_for(char_block));
        events.push(end_event_for(char_block));
        self.block_idx += 1;

        // Following Dialogue/Parenthetical
        while self.block_idx < self.doc.blocks.len() {
            let next = &self.doc.blocks[self.block_idx];
            match next {
                Block::Dialogue { .. } | Block::Parenthetical { .. } => {
                    events.push(start_event_for(next));
                    events.push(text_event_for(next));
                    events.push(end_event_for(next));
                    self.block_idx += 1;
                }
                _ => break,
            }
        }

        events.push(Event::EndDialogueBlock);

        // Return the first, push the rest in reverse for pop().
        let first = events.remove(0);
        events.reverse();
        self.pending = events;
        first
    }
}

fn start_event_for<'a>(block: &'a Block) -> Event<'a> {
    match block {
        Block::SceneHeading { .. } => Event::StartSceneHeading,
        Block::Action { .. } => Event::StartAction,
        Block::Character { dual, .. } => Event::StartCharacter { dual: *dual },
        Block::Dialogue { .. } => Event::StartDialogue,
        Block::Parenthetical { .. } => Event::StartParenthetical,
        Block::Transition { .. } => Event::StartTransition,
        Block::Centered { .. } => Event::StartCentered,
        Block::Lyric { .. } => Event::StartLyric,
        Block::Note { .. } => Event::StartNote,
        Block::Synopsis { .. } => Event::StartSynopsis,
        Block::Section { level, .. } => Event::StartSection { level: *level },
        Block::PageBreak { .. } => Event::PageBreak,
        Block::Boneyard { .. } => Event::StartBoneyard,
    }
}

fn text_event_for<'a>(block: &'a Block) -> Event<'a> {
    match block {
        Block::SceneHeading { text, .. }
        | Block::Action { text, .. }
        | Block::Dialogue { text, .. }
        | Block::Parenthetical { text, .. }
        | Block::Transition { text, .. }
        | Block::Centered { text, .. }
        | Block::Lyric { text, .. }
        | Block::Note { text, .. }
        | Block::Synopsis { text, .. }
        | Block::Section { text, .. }
        | Block::Boneyard { text, .. } => Event::Text(Cow::Borrowed(text.as_str())),
        Block::Character { name, .. } => Event::Text(Cow::Borrowed(name.as_str())),
        Block::PageBreak { .. } => Event::Text(Cow::Borrowed("")),
    }
}

fn end_event_for<'a>(block: &'a Block) -> Event<'a> {
    match block {
        Block::SceneHeading { .. } => Event::EndSceneHeading,
        Block::Action { .. } => Event::EndAction,
        Block::Character { .. } => Event::EndCharacter,
        Block::Dialogue { .. } => Event::EndDialogue,
        Block::Parenthetical { .. } => Event::EndParenthetical,
        Block::Transition { .. } => Event::EndTransition,
        Block::Centered { .. } => Event::EndCentered,
        Block::Lyric { .. } => Event::EndLyric,
        Block::Note { .. } => Event::EndNote,
        Block::Synopsis { .. } => Event::EndSynopsis,
        Block::Section { .. } => Event::EndSection,
        Block::PageBreak { .. } => Event::PageBreak, // leaf — no end
        Block::Boneyard { .. } => Event::EndBoneyard,
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> OwnedEventIter {
    let (doc, _diags) = crate::parse::parse(input);
    OwnedEventIter::new(doc)
}

/// Owned event iterator that owns the document.
pub struct OwnedEventIter {
    doc: FountainDoc,
    phase: Phase,
    block_idx: usize,
    inner: InnerState,
    meta_keys: Vec<String>,
    meta_idx: usize,
    started: bool,
    finished: bool,
    pending: Vec<OwnedEvent>,
}

impl OwnedEventIter {
    fn new(doc: FountainDoc) -> Self {
        let meta_keys: Vec<String> = doc.metadata.keys().cloned().collect();
        OwnedEventIter {
            doc,
            phase: Phase::Metadata,
            block_idx: 0,
            inner: InnerState::Start,
            meta_keys,
            meta_idx: 0,
            started: false,
            finished: false,
            pending: Vec::new(),
        }
    }
}

impl Iterator for OwnedEventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if let Some(ev) = self.pending.pop() {
            return Some(ev);
        }

        if !self.started {
            self.started = true;
            return Some(Event::StartDocument);
        }

        if self.finished {
            return None;
        }

        if self.phase == Phase::Metadata {
            if self.meta_idx < self.meta_keys.len() {
                let key = self.meta_keys[self.meta_idx].clone();
                let value = self.doc.metadata[&key].clone();
                self.meta_idx += 1;
                return Some(Event::Metadata {
                    key: Cow::Owned(key),
                    value: Cow::Owned(value),
                });
            }
            self.phase = Phase::Blocks;
        }

        if self.phase == Phase::Blocks {
            if self.block_idx >= self.doc.blocks.len() {
                self.phase = Phase::Done;
                self.finished = true;
                return Some(Event::EndDocument);
            }

            let block = &self.doc.blocks[self.block_idx];

            match self.inner {
                InnerState::Start => {
                    if matches!(block, Block::Character { .. }) {
                        return Some(self.expand_dialogue_block());
                    }

                    self.inner = InnerState::End;
                    let ev = start_event_for(block).into_owned();
                    // For non-PageBreak blocks, also emit text
                    if !matches!(block, Block::PageBreak { .. }) {
                        // Push the text event as pending (will be popped next)
                        // Actually for the simple Start/Text/End pattern we handle
                        // Text in End state.
                    }
                    Some(ev)
                }
                InnerState::End => {
                    self.inner = InnerState::Start;
                    self.block_idx += 1;
                    // For leaf PageBreak, we already emitted it
                    if matches!(block, Block::PageBreak { .. }) {
                        // PageBreak is a leaf, skip end event
                        return self.next();
                    }
                    // Push text event and end event: text first (pending reversed)
                    let end = end_event_for(block).into_owned();
                    let text = text_event_for(block).into_owned();
                    self.pending.push(end);
                    Some(text)
                }
            }
        } else {
            self.finished = true;
            Some(Event::EndDocument)
        }
    }
}

impl OwnedEventIter {
    fn expand_dialogue_block(&mut self) -> OwnedEvent {
        let mut events: Vec<OwnedEvent> = Vec::new();

        events.push(Event::StartDialogueBlock);

        let char_block = &self.doc.blocks[self.block_idx];
        events.push(start_event_for(char_block).into_owned());
        events.push(text_event_for(char_block).into_owned());
        events.push(end_event_for(char_block).into_owned());
        self.block_idx += 1;

        while self.block_idx < self.doc.blocks.len() {
            let next = &self.doc.blocks[self.block_idx];
            match next {
                Block::Dialogue { .. } | Block::Parenthetical { .. } => {
                    events.push(start_event_for(next).into_owned());
                    events.push(text_event_for(next).into_owned());
                    events.push(end_event_for(next).into_owned());
                    self.block_idx += 1;
                }
                _ => break,
            }
        }

        events.push(Event::EndDialogueBlock);

        let first = events.remove(0);
        events.reverse();
        self.pending = events;
        first
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_scene_heading() {
        let evs: Vec<_> = events("INT. COFFEE SHOP - DAY").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartSceneHeading)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndSceneHeading)));
    }

    #[test]
    fn test_events_dialogue() {
        let evs: Vec<_> = events("JOHN\nHello there.").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartDialogueBlock)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartCharacter { .. })));
        assert!(evs.iter().any(|e| matches!(e, Event::StartDialogue)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndDialogueBlock)));
    }

    #[test]
    fn test_events_transition() {
        let evs: Vec<_> = events("CUT TO:").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartTransition)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndTransition)));
    }

    #[test]
    fn test_events_page_break() {
        let evs: Vec<_> = events("===").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::PageBreak)));
    }

    #[test]
    fn test_events_boneyard() {
        let evs: Vec<_> = events("/* This is a comment */").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBoneyard)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBoneyard)));
    }

    #[test]
    fn test_events_metadata() {
        let evs: Vec<_> = events("Title: My Script\n\nINT. HOUSE - DAY").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::Metadata { key, .. } if key == "title")));
    }
}
