//! Org-mode emitter (builder).

use crate::ast::{Block, CheckboxState, Inline, ListItem, ListItemContent, OrgDoc, TableRow};

/// Build an Org-mode string from an [`OrgDoc`].
pub fn build(doc: &OrgDoc) -> String {
    let mut ctx = BuildContext::new();

    // Emit metadata as #+KEY: value lines
    for (key, value) in &doc.metadata {
        ctx.write("#+");
        ctx.write(&key.to_uppercase());
        ctx.write(": ");
        ctx.write(value);
        ctx.write("\n");
    }
    if !doc.metadata.is_empty() && !doc.blocks.is_empty() {
        ctx.write("\n");
    }

    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }

    // Ensure trailing newline
    if !ctx.output.ends_with('\n') {
        ctx.output.push('\n');
    }

    ctx.output
}

struct BuildContext {
    output: String,
    list_depth: usize,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_depth: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn ensure_blank_line(&mut self) {
        let trimmed = self.output.trim_end();
        let len = trimmed.len();
        self.output.truncate(len);
        self.output.push_str("\n\n");
    }

    fn ensure_newline(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.ensure_blank_line();
        }

        Block::Heading { level, todo, priority, tags, properties, scheduled, deadline, inlines, .. } => {
            ctx.ensure_newline();
            for _ in 0..*level {
                ctx.write("*");
            }
            ctx.write(" ");
            if let Some(kw) = todo {
                ctx.write(kw);
                ctx.write(" ");
            }
            if let Some(p) = priority {
                ctx.write("[#");
                ctx.write(p);
                ctx.write("] ");
            }
            build_inlines(inlines, ctx);
            if !tags.is_empty() {
                ctx.write("    :");
                ctx.write(&tags.join(":"));
                ctx.write(":");
            }
            ctx.ensure_newline();
            // Emit :PROPERTIES: drawer if present
            if !properties.is_empty() {
                ctx.write(":PROPERTIES:\n");
                for (k, v) in properties {
                    ctx.write(":");
                    ctx.write(k);
                    ctx.write(": ");
                    ctx.write(v);
                    ctx.write("\n");
                }
                ctx.write(":END:\n");
            }
            // Emit SCHEDULED:/DEADLINE: planning lines
            if scheduled.is_some() || deadline.is_some() {
                if let Some(s) = scheduled {
                    ctx.write("SCHEDULED: ");
                    ctx.write(s);
                    if deadline.is_some() {
                        ctx.write(" ");
                    } else {
                        ctx.write("\n");
                    }
                }
                if let Some(d) = deadline {
                    ctx.write("DEADLINE: ");
                    ctx.write(d);
                    ctx.write("\n");
                }
            }
            ctx.ensure_blank_line();
        }

        Block::CodeBlock { language, header_args, name, content, .. } => {
            ctx.ensure_newline();
            if let Some(nm) = name {
                ctx.write("#+NAME: ");
                ctx.write(nm);
                ctx.write("\n");
            }
            if let Some(lang) = language {
                ctx.write("#+BEGIN_SRC ");
                ctx.write(lang);
                if let Some(args) = header_args {
                    ctx.write(" ");
                    ctx.write(args);
                }
                ctx.write("\n");
            } else {
                ctx.write("#+BEGIN_SRC\n");
            }
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("#+END_SRC\n\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.ensure_newline();
            ctx.write("#+BEGIN_QUOTE\n");
            for child in children {
                build_block(child, ctx);
            }
            ctx.write("#+END_QUOTE\n\n");
        }

        Block::List { ordered, start, items, .. } => {
            ctx.list_depth += 1;
            let mut counter = start.map(|s| s as i32).unwrap_or(1);
            let mut first_item = true;
            for item in items {
                let emit_counter = if *ordered && first_item { *start } else { None };
                build_list_item(item, *ordered, &mut counter, emit_counter, ctx);
                first_item = false;
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.ensure_newline();
            }
        }

        Block::Table { rows, .. } => {
            build_table(rows, ctx);
        }

        Block::HorizontalRule { .. } => {
            ctx.ensure_newline();
            ctx.write("-----\n\n");
        }

        Block::DefinitionList { items, .. } => {
            for item in items {
                ctx.write("- ");
                build_inlines(&item.term, ctx);
                ctx.write(" :: ");
                build_inlines(&item.desc, ctx);
                ctx.ensure_newline();
            }
            ctx.ensure_newline();
        }

        Block::Div { inlines, .. } => {
            build_inlines(inlines, ctx);
        }

        Block::RawBlock { format, content, .. } => {
            if format == "org" {
                ctx.write(content);
            }
        }

        Block::Figure { children, .. } => {
            for child in children {
                build_block(child, ctx);
            }
        }

        Block::Caption { inlines, .. } => {
            ctx.write("#+CAPTION: ");
            build_inlines(inlines, ctx);
            ctx.ensure_newline();
        }

        Block::Unknown { .. } => {
            // Unknown block — silently skip (diagnostic already emitted at parse time)
        }
    }
}

fn build_list_item(
    item: &ListItem,
    ordered: bool,
    counter: &mut i32,
    emit_start_cookie: Option<u64>,
    ctx: &mut BuildContext,
) {
    let indent = "  ".repeat(ctx.list_depth - 1);
    ctx.write(&indent);

    if ordered {
        if let Some(start_n) = emit_start_cookie {
            ctx.write(&format!("{}. [@{}] ", counter, start_n));
        } else {
            ctx.write(&format!("{}. ", counter));
        }
        *counter += 1;
    } else {
        ctx.write("- ");
    }

    // Emit checkbox prefix if present
    if let Some(checkbox) = item.checkbox {
        match checkbox {
            CheckboxState::Unchecked => ctx.write("[ ] "),
            CheckboxState::Checked => ctx.write("[X] "),
            CheckboxState::Partial => ctx.write("[-] "),
        }
    }

    let mut first = true;
    for child in &item.children {
        match child {
            ListItemContent::Inline(inlines) => {
                build_inlines(inlines, ctx);
                ctx.ensure_newline();
            }
            ListItemContent::Block(block) => {
                if first {
                    // If first child is a paragraph-like block, emit inline
                    if let Block::Paragraph { inlines, .. } = block {
                        build_inlines(inlines, ctx);
                        ctx.ensure_newline();
                    } else {
                        ctx.ensure_newline();
                        build_block(block, ctx);
                    }
                } else if let Block::Paragraph { inlines, .. } = block {
                    let content_indent = "  ".repeat(ctx.list_depth);
                    ctx.write(&content_indent);
                    build_inlines(inlines, ctx);
                    ctx.ensure_newline();
                } else if let Block::List { .. } = block {
                    ctx.ensure_newline();
                    build_block(block, ctx);
                } else {
                    build_block(block, ctx);
                }
            }
        }
        first = false;
    }
}

fn build_table(rows: &[TableRow], ctx: &mut BuildContext) {
    ctx.ensure_newline();

    if rows.is_empty() {
        ctx.ensure_blank_line();
        return;
    }

    // Build string representations of all cells
    let string_rows: Vec<(Vec<String>, bool)> = rows
        .iter()
        .map(|row| {
            let cells: Vec<String> = row
                .cells
                .iter()
                .map(|cell| {
                    let mut cell_ctx = BuildContext::new();
                    build_inlines(cell, &mut cell_ctx);
                    cell_ctx.output.trim().to_string()
                })
                .collect();
            (cells, row.is_header)
        })
        .collect();

    // Calculate column widths
    let num_cols = string_rows.iter().map(|(r, _)| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; num_cols];
    for (row, _) in &string_rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    // Find where the first header row ends (for separator placement)
    let first_header_idx = string_rows.iter().position(|(_, is_hdr)| *is_hdr);
    let separator_after = first_header_idx.map(|_| 0usize); // separator after row 0 if there's a header

    let total_rows = string_rows.len();
    for (idx, (row, _is_header)) in string_rows.iter().enumerate() {
        ctx.write("|");
        for (i, cell) in row.iter().enumerate() {
            ctx.write(" ");
            ctx.write(cell);
            let padding = col_widths[i].saturating_sub(cell.len());
            ctx.write(&" ".repeat(padding));
            ctx.write(" |");
        }
        ctx.write("\n");

        // Add separator after first row if there are multiple rows
        if separator_after.is_some() && idx == 0 && total_rows > 1 {
            ctx.write("|");
            for width in &col_widths {
                ctx.write("-");
                ctx.write(&"-".repeat(*width));
                ctx.write("-+");
            }
            // Fix the last + to |
            if !col_widths.is_empty() {
                ctx.output.pop();
                ctx.write("|");
            }
            ctx.write("\n");
        }
    }

    ctx.ensure_blank_line();
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text { text, .. } => ctx.write(text),

        Inline::Bold(children, _) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children, _) => {
            ctx.write("/");
            build_inlines(children, ctx);
            ctx.write("/");
        }

        Inline::Underline(children, _) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Strikethrough(children, _) => {
            ctx.write("+");
            build_inlines(children, ctx);
            ctx.write("+");
        }

        Inline::Code(s, _) => {
            ctx.write("=");
            ctx.write(s);
            ctx.write("=");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("][");
            build_inlines(children, ctx);
            ctx.write("]]");
        }

        Inline::Image { url, .. } => {
            ctx.write("[[");
            if !url.starts_with("file:") && !url.starts_with("http") {
                ctx.write("file:");
            }
            ctx.write(url);
            ctx.write("]]");
        }

        Inline::LineBreak { .. } => ctx.write("\\\\\n"),

        Inline::SoftBreak { .. } => ctx.write("\n"),

        Inline::Superscript(children, _) => {
            ctx.write("^{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Subscript(children, _) => {
            ctx.write("_{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::FootnoteRef { label, .. } => {
            ctx.write("[fn:");
            ctx.write(label);
            ctx.write("]");
        }

        Inline::FootnoteDefinition { label, children, .. } => {
            ctx.write("[fn:");
            ctx.write(label);
            ctx.write(": ");
            build_inlines(children, ctx);
            ctx.write("]");
        }

        Inline::MathInline { source, .. } => {
            ctx.write("$");
            ctx.write(source);
            ctx.write("$");
        }

        Inline::Timestamp { active, value, .. } => {
            if *active {
                ctx.write("<");
                ctx.write(value);
                ctx.write(">");
            } else {
                ctx.write("[");
                ctx.write(value);
                ctx.write("]");
            }
        }

        Inline::ExportSnippet { backend, value, .. } => {
            ctx.write("@@");
            ctx.write(backend);
            ctx.write(":");
            ctx.write(value);
            ctx.write("@@");
        }
    }
}

