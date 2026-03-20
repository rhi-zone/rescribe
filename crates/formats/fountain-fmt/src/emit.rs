//! Fountain emitter: AST → text.

use std::collections::BTreeMap;

use crate::ast::{Block, FountainDoc};

// ── Public API ────────────────────────────────────────────────────────────────

/// Build a Fountain string from a [`FountainDoc`].
pub fn emit(doc: &FountainDoc) -> String {
    let mut ctx = BuildContext::new();

    // Emit title page metadata
    emit_title_page(&doc.metadata, &mut ctx);

    // Emit content
    emit_blocks(&doc.blocks, &mut ctx);

    ctx.output
}

// ── Build context ─────────────────────────────────────────────────────────────

struct BuildContext {
    output: String,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn ensure_blank_line(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            if self.output.ends_with('\n') {
                self.output.push('\n');
            } else {
                self.output.push_str("\n\n");
            }
        }
    }
}

fn emit_title_page(metadata: &BTreeMap<String, String>, ctx: &mut BuildContext) {
    if metadata.is_empty() {
        return;
    }

    // Standard title page fields in order
    let field_order = [
        "title",
        "credit",
        "author",
        "authors",
        "source",
        "draft_date",
        "contact",
        "copyright",
        "notes",
    ];

    let mut has_output = false;
    for field in field_order {
        if let Some(value) = metadata.get(field) {
            let display_key = field
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            ctx.write(&display_key);
            ctx.write(": ");
            ctx.writeln(value);
            has_output = true;
        }
    }

    // Add any non-standard fields (shouldn't happen, but just in case)
    for (key, value) in metadata.iter() {
        if !field_order.contains(&key.as_str()) {
            let display_key = key
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            ctx.write(&display_key);
            ctx.write(": ");
            ctx.writeln(value);
            has_output = true;
        }
    }

    if has_output {
        ctx.writeln("");
    }
}

fn emit_blocks(blocks: &[Block], ctx: &mut BuildContext) {
    for block in blocks {
        emit_block(block, ctx);
    }
}

fn emit_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::SceneHeading { text, .. } => {
            ctx.ensure_blank_line();
            ctx.writeln(text);
        }

        Block::Action { text, .. } => {
            ctx.ensure_blank_line();
            ctx.writeln(text);
        }

        Block::Character { name, dual, .. } => {
            ctx.ensure_blank_line();
            if *dual {
                ctx.writeln(&format!("{} ^", name.to_uppercase()));
            } else {
                ctx.writeln(&name.to_uppercase());
            }
        }

        Block::Dialogue { text, .. } => {
            ctx.writeln(text);
        }

        Block::Parenthetical { text, .. } => {
            ctx.writeln(text);
        }

        Block::Transition { text, .. } => {
            ctx.ensure_blank_line();
            // If it doesn't look like a standard transition, force it with >
            if !text.to_uppercase().ends_with("TO:") {
                ctx.write(">");
            }
            ctx.writeln(&text.to_uppercase());
        }

        Block::Centered { text, .. } => {
            ctx.ensure_blank_line();
            ctx.write(">");
            ctx.write(text);
            ctx.writeln("<");
        }

        Block::Lyric { text, .. } => {
            ctx.write("~");
            ctx.writeln(text);
        }

        Block::Note { text, .. } => {
            ctx.write("[[");
            ctx.write(text);
            ctx.writeln("]]");
        }

        Block::Synopsis { text, .. } => {
            ctx.write("= ");
            ctx.writeln(text);
        }

        Block::Section { level, text, .. } => {
            ctx.ensure_blank_line();
            ctx.write(&"#".repeat(*level));
            ctx.write(" ");
            ctx.writeln(text);
        }

        Block::PageBreak { .. } => {
            ctx.ensure_blank_line();
            ctx.writeln("===");
        }
    }
}
