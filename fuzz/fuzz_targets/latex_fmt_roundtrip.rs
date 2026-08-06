#![no_main]

//! latex-fmt native AST roundtrip fuzz target.
//!
//! Direction: arbitrary_latex_ast -> emit -> parse -> assert equality
//! (per CLAUDE.md: starts from the format crate's own `Ast`/`LatexDoc`
//! type, not the IR).
//!
//! # Scoping the generated AST space
//!
//! `latex-fmt`'s resolution model (see `crate::parse`'s module docs) means
//! a `Node::Command`/`Node::Environment` (resolved) is only reachable from
//! `parse()` when preceded, in the same or an enclosing scope, by a
//! matching in-document `\newcommand`/`\newenvironment`-family definition
//! — an arbitrary hand-built `Command` node with no such preceding
//! definer would, after `emit()` + `parse()`, come back as an unresolved
//! `ControlSymbol` instead (correct, documented behavior — see
//! `crate::parse`'s "resolution model" docs — not a roundtrip bug). This
//! generator therefore only ever produces a resolved `Command`/
//! `Environment` use paired with a `\newcommand`/`\newenvironment`
//! definer immediately before it in the same generated sequence, so the
//! pairing is self-consistent under re-parsing. Similarly,
//! `Node::MathDisplay { env: Some(_), .. }` is not currently reachable
//! from `parse()` at all (no environment-name table decides "this looks
//! like math" anymore, per the resolved design — see TODO.md) and is
//! excluded from generation; `env: None` (`$$...$$`) is generated
//! normally.

use latex_fmt::{Diagnostic, LatexDoc, Node, Span};
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Emit as _, Parse as _};

struct Gen<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Gen<'a> {
    fn new(data: &'a [u8]) -> Self {
        Gen { data, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }

    /// Lowercase ASCII letters only, 1..=n chars — always safe as plain
    /// text (no tokenizer-special bytes), a control-sequence name
    /// fragment, or an environment name.
    fn word(&mut self, n: usize) -> String {
        let len = (self.byte() as usize % n.max(1)) + 1;
        let s: String = (0..len)
            .map(|_| ((self.byte() % 26) + b'a') as char)
            .collect();
        if s.is_empty() { "x".to_string() } else { s }
    }

    /// Like `word`, but re-rolled if it happens to collide with one of
    /// `parse.rs`'s nine reserved definer names (`def`, `newcommand`, ...).
    /// A real document can't `\newcommand` a name that collides with a
    /// TeX/LaTeX-kernel primitive/definer (real TeX would refuse — `\def`
    /// isn't a valid `\newcommand` target), so a generated AST that picks
    /// one of these as a *user* macro/environment name isn't representative
    /// of any real document; excluding them keeps the generated space
    /// matched to what `parse()` actually needs to round-trip.
    fn definable_name(&mut self, n: usize) -> String {
        const RESERVED: &[&str] = &[
            "def",
            "edef",
            "gdef",
            "xdef",
            "newcommand",
            "renewcommand",
            "providecommand",
            "newenvironment",
            "renewenvironment",
        ];
        loop {
            let w = self.word(n);
            if !RESERVED.contains(&w.as_str()) {
                return w;
            }
        }
    }

    /// Uppercase-letter names for `leaf()`'s `ControlSymbol` generation —
    /// deliberately disjoint from `definable_name`'s all-lowercase pool
    /// (used for `\newcommand`/`\newenvironment` targets), so a leaf-level
    /// "arbitrary unresolved control sequence" can never accidentally
    /// collide with a name the generator has locally defined elsewhere in
    /// the same document. A collision would be a real correctness issue
    /// were it possible: `Node::ControlSymbol { name, .. }` specifically
    /// means "unresolved" — if `name` happened to match an in-scope
    /// `\newcommand`/`\newenvironment` definition, re-parsing the emitted
    /// source would correctly resolve it (per `crate::parse`'s resolution
    /// model), producing a `Command`/`Environment` node instead and
    /// breaking round-trip. That would be a generator bug, not a parser
    /// bug — disjoint name pools sidestep it entirely rather than requiring
    /// the generator to thread scope state through `leaf()`.
    fn control_symbol_name(&mut self, n: usize) -> String {
        let len = (self.byte() as usize % n.max(1)) + 1;
        let s: String = (0..len)
            .map(|_| ((self.byte() % 26) + b'A') as char)
            .collect();
        if s.is_empty() { "X".to_string() } else { s }
    }

    fn leaf(&mut self, depth: u8) -> Node {
        match self.byte() % 8 {
            0 => Node::Text {
                value: self.word(6),
                span: Span::NONE,
            },
            1 => Node::ControlSymbol {
                name: self.control_symbol_name(5),
                span: Span::NONE,
            },
            2 => Node::MathInline {
                source: self.word(5),
                span: Span::NONE,
            },
            3 => Node::MathDisplay {
                source: self.word(5),
                env: None,
                span: Span::NONE,
            },
            4 => Node::AlignTab { span: Span::NONE },
            5 => Node::RowBreak {
                spacing: None,
                span: Span::NONE,
            },
            6 => Node::Verb {
                star: false,
                delim: '|',
                content: self.word(5),
                span: Span::NONE,
            },
            _ if depth < 3 => {
                let n = (self.byte() % 3) as usize;
                Node::Group {
                    body: (0..n).map(|_| self.leaf(depth + 1)).collect(),
                    span: Span::NONE,
                }
            }
            _ => Node::Text {
                value: self.word(4),
                span: Span::NONE,
            },
        }
    }

    /// A `\newcommand{\name}[nargs]{body}` definer immediately followed by
    /// a matching, self-consistent resolved use `\name{a1}...{aN}` — see
    /// module doc for why the pairing must stay together.
    fn defined_command_use(&mut self, depth: u8) -> Vec<Node> {
        let name = self.definable_name(6);
        let nargs = self.byte() % 3; // 0..=2, keep bounded
        let body: Vec<Node> = (0..(self.byte() % 3))
            .map(|_| self.leaf(depth + 1))
            .collect();
        let definer = Node::Command {
            name: "newcommand".to_string(),
            star: false,
            opt: vec![vec![Node::Text {
                value: nargs.to_string(),
                span: Span::NONE,
            }]],
            args: vec![
                vec![Node::ControlSymbol {
                    name: name.clone(),
                    span: Span::NONE,
                }],
                body,
            ],
            span: Span::NONE,
        };
        let use_args: Vec<Vec<Node>> = (0..nargs)
            .map(|_| {
                (0..(self.byte() % 2))
                    .map(|_| self.leaf(depth + 1))
                    .collect()
            })
            .collect();
        let use_node = Node::Command {
            name,
            star: false,
            opt: vec![],
            args: use_args,
            span: Span::NONE,
        };
        vec![definer, use_node]
    }

    /// `\newenvironment{name}{begin}{end}` (nargs fixed at 0 to keep this
    /// pairing simple — argument-taking custom environments are already
    /// covered by unit tests in `parse.rs`) followed by a matching use.
    fn defined_environment_use(&mut self, depth: u8) -> Vec<Node> {
        let name = self.definable_name(6);
        let begin: Vec<Node> = (0..(self.byte() % 2))
            .map(|_| self.leaf(depth + 1))
            .collect();
        let end: Vec<Node> = (0..(self.byte() % 2))
            .map(|_| self.leaf(depth + 1))
            .collect();
        let definer = Node::Command {
            name: "newenvironment".to_string(),
            star: false,
            opt: vec![],
            args: vec![
                vec![Node::Text {
                    value: name.clone(),
                    span: Span::NONE,
                }],
                begin,
                end,
            ],
            span: Span::NONE,
        };
        let body: Vec<Node> = (0..(self.byte() % 3))
            .map(|_| self.leaf(depth + 1))
            .collect();
        let use_node = Node::Environment {
            name,
            star: false,
            opt: vec![],
            args: vec![],
            body,
            span: Span::NONE,
        };
        vec![definer, use_node]
    }

    fn top_level(&mut self) -> Vec<Node> {
        let count = (self.byte() % 6) + 1;
        let mut out = Vec::new();
        for _ in 0..count {
            match self.byte() % 4 {
                0 => out.extend(self.defined_command_use(0)),
                1 => out.extend(self.defined_environment_use(0)),
                _ => out.push(self.leaf(0)),
            }
        }
        out
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let mut g = Gen::new(data);
    let nodes = g.top_level();
    let mut doc = LatexDoc { nodes };

    let emitted_bytes = doc.emit();
    let (mut doc2, diags): (LatexDoc, Vec<Diagnostic>) = LatexDoc::parse(&emitted_bytes);

    // `normalize()` merges adjacent `Text` siblings the generator above
    // can produce (e.g. two consecutive plain-text leaves with nothing
    // tokenizer-special between them) into the single merged run `parse()`
    // always produces — see `LatexDoc::normalize`'s doc comment for why
    // this is the correct comparison, not a workaround.
    doc.normalize().strip_spans();
    doc2.strip_spans();

    assert_eq!(
        doc.nodes,
        doc2.nodes,
        "latex-fmt roundtrip mismatch\n  emitted: {:?}\n  diags: {diags:?}",
        String::from_utf8_lossy(&emitted_bytes)
    );
});
