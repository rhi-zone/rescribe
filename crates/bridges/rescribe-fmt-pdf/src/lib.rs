//! PDF reader for rescribe (no writer: PDF generation is out of scope — see
//! crate-level rationale in TODO.md/docs/format-audit.md).
//!
//! Parses PDF files into rescribe's document IR.
//!
//! # Limitations
//!
//! PDF is fundamentally a visual/layout format, not a semantic format.
//! This reader extracts text content and applies a font-size heuristic to
//! distinguish headings from body text, but it cannot reliably determine:
//! - List structure
//! - Table structure
//! - Emphasis/bold/italic (font changes, not semantic markup)
//!
//! Heading detection works by driving `pdf_extract`'s low-level `OutputDev`
//! trait directly (rather than its convenience `extract_text_by_pages` API)
//! to observe each character's transformed font size and position. Runs of
//! text are grouped into lines and blocks using the same vertical-gap
//! heuristic `pdf_extract`'s own `PlainTextOutput` uses to decide line
//! breaks, extended with a larger-gap threshold for block/paragraph breaks.
//! The most common (mode) font size across the document is treated as the
//! "body text" size; a block whose first character is meaningfully larger
//! than that mode becomes a `heading` node instead of a `paragraph` node.
//!
//! This is a heuristic over visual presentation, not a semantic PDF
//! structure reader (PDF/UA tagged structure trees are not consulted). It
//! can mis-detect headings in documents that don't follow the "headings are
//! bigger than body text" convention, and it cannot infer heading levels
//! from anything but relative font size. For better structure extraction
//! from PDFs, consider using an OCR-based approach, a specialized PDF
//! analysis tool that can infer document structure, or (where available) a
//! reader of the PDF/UA logical structure tree.

use euclid::{Transform2D, vec2};
use pdf_extract::{MediaBox, OutputError, Transform};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, ParseOptions, Properties,
    Severity, SourceInfo, WarningKind,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;

/// A vertical gap larger than this multiple of the transformed font size
/// (but not large enough to be a block break) is treated as a line break
/// within the same block. Matches the threshold `pdf_extract`'s own
/// `PlainTextOutput` uses to decide when to emit a newline.
const LINE_BREAK_Y_FACTOR: f64 = 1.5;

/// A vertical gap larger than this multiple of the transformed font size is
/// treated as a paragraph/block break (roughly: two blank lines' worth of
/// space, wider than ordinary line spacing).
const BLOCK_BREAK_Y_FACTOR: f64 = 2.5;

/// A block whose first character's font size is at least this multiple of
/// the document's body-text mode font size is treated as a heading rather
/// than a paragraph. 1.15x is a conservative threshold: it's larger than
/// typical anti-aliasing/rounding noise in extracted font sizes, but small
/// enough to catch subheadings that are only slightly larger than body
/// text.
const HEADING_SIZE_RATIO: f64 = 1.15;

/// Parse PDF bytes into a rescribe Document.
pub fn parse(input: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse PDF with custom options.
pub fn parse_with_options(
    input: &[u8],
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let mut warnings = Vec::new();

    let mut pdf_doc = pdf_extract::Document::load_mem(input)
        .map_err(|e| ParseError::Invalid(format!("PDF extraction failed: {}", e)))?;

    // Mirror pdf_extract's own `extract_text_from_mem_by_pages`: opportunistically
    // decrypt with an empty password (many "encrypted" PDFs use empty-password
    // encryption purely to set permission flags, not to actually restrict access).
    if pdf_doc.is_encrypted() {
        let _ = pdf_doc.decrypt("");
    }

    let mut collector = StructuredCollector::new();
    pdf_extract::output_doc(&pdf_doc, &mut collector)
        .map_err(|e| ParseError::Invalid(format!("PDF extraction failed: {}", e)))?;
    let (blocks, font_size_histogram) = collector.finish();

    let body_mode_size = body_text_mode_size(&font_size_histogram);

    let mut doc_children = Vec::new();
    let mut last_page = None;

    for block in &blocks {
        if let Some(prev_page) = last_page
            && block.page_num != prev_page
        {
            let page_break = Node::new(node::HORIZONTAL_RULE).prop(prop::LAYOUT_PAGE_BREAK, true);
            doc_children.push(page_break);
        }
        last_page = Some(block.page_num);

        if block.text.is_empty() {
            continue;
        }

        let is_heading = body_mode_size > 0.0
            && block.first_char_font_size >= body_mode_size * HEADING_SIZE_RATIO;

        if is_heading {
            let level = heading_level(block.first_char_font_size, body_mode_size);
            let heading = Node::new(node::HEADING)
                .prop(prop::LEVEL, level as i64)
                .prop(prop::STYLE_SIZE, block.first_char_font_size)
                .child(text_node(&block.text));
            doc_children.push(heading);
        } else {
            let para = Node::new(node::PARAGRAPH).child(text_node(&block.text));
            doc_children.push(para);
        }
    }

    // Add warning about structural loss
    warnings.push(FidelityWarning::new(
        Severity::Major,
        WarningKind::FeatureLost("PDF structure".into()),
        "PDF is a visual format; list, table, and emphasis structure cannot be reliably \
         extracted. Heading detection is a font-size heuristic (largest text in a block vs. \
         the document's body-text mode size), not a reader of PDF/UA tagged structure -- it \
         can misclassify text in documents that don't size headings larger than body text.",
    ));

    let doc_node = Node::new(node::DOCUMENT).children(doc_children);

    let document = Document {
        content: doc_node,
        resources: Default::default(),
        metadata: Properties::new(),
        source: Some(SourceInfo {
            format: "pdf".to_string(),
            metadata: Properties::new(),
        }),
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

/// A block of text (one or more wrapped lines with no large vertical gap
/// between them), along with the font size of its first character and the
/// page it appeared on.
struct Block {
    text: String,
    first_char_font_size: f64,
    page_num: u32,
}

/// Drives `pdf_extract`'s `OutputDev` to collect text grouped into blocks,
/// tracking each character's transformed font size and position so callers
/// can apply layout heuristics that plain extracted text can't express.
struct StructuredCollector {
    blocks: Vec<Block>,
    current_page: u32,
    flip_ctm: Transform,

    current_line: String,
    current_block: String,
    block_start_font_size: Option<f64>,

    /// Font size (transformed) of every character seen, rounded to the
    /// nearest 0.5pt, used to compute the document's body-text mode size.
    font_size_histogram: HashMap<i64, usize>,

    last_end: f64,
    last_y: f64,
    /// Set by `begin_word`; PDF content streams group characters into
    /// words, and word boundaries are where layout heuristics (spacing,
    /// line/block breaks) are evaluated -- mirrors `PlainTextOutput`.
    first_char_of_word: bool,
}

impl StructuredCollector {
    fn new() -> Self {
        StructuredCollector {
            blocks: Vec::new(),
            current_page: 0,
            flip_ctm: Transform2D::identity(),
            current_line: String::new(),
            current_block: String::new(),
            block_start_font_size: None,
            font_size_histogram: HashMap::new(),
            last_end: 100_000.0,
            last_y: 0.0,
            first_char_of_word: false,
        }
    }

    fn flush_line(&mut self) {
        if self.current_line.trim().is_empty() {
            self.current_line.clear();
            return;
        }
        if !self.current_block.is_empty() {
            self.current_block.push(' ');
        }
        self.current_block.push_str(self.current_line.trim());
        self.current_line.clear();
    }

    fn flush_block(&mut self) {
        self.flush_line();
        if !self.current_block.trim().is_empty() {
            self.blocks.push(Block {
                text: normalize_whitespace(self.current_block.trim()),
                first_char_font_size: self.block_start_font_size.unwrap_or(0.0),
                page_num: self.current_page,
            });
        }
        self.current_block.clear();
        self.block_start_font_size = None;
    }

    fn finish(mut self) -> (Vec<Block>, HashMap<i64, usize>) {
        self.flush_block();
        (self.blocks, self.font_size_histogram)
    }
}

impl pdf_extract::OutputDev for StructuredCollector {
    fn begin_page(
        &mut self,
        page_num: u32,
        media_box: &MediaBox,
        _art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        // A new page is at least as strong a signal as a block break: flush
        // whatever was accumulated on the previous page before starting.
        self.flush_block();
        self.current_page = page_num;
        self.flip_ctm = Transform2D::row_major(1., 0., 0., -1., 0., media_box.ury - media_box.lly);
        self.last_end = 100_000.0;
        self.last_y = 0.0;
        self.first_char_of_word = false;
        Ok(())
    }

    fn end_page(&mut self) -> Result<(), OutputError> {
        self.flush_line();
        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError> {
        let position = trm.post_transform(&self.flip_ctm);
        let font_size_vec = trm.transform_vector(vec2(font_size, font_size));
        // Length of one side of a square with the same area as the
        // transformed (possibly non-uniformly scaled) font size box --
        // same approach `pdf_extract`'s own output devices use.
        let transformed_font_size = (font_size_vec.x * font_size_vec.y).sqrt();
        let (x, y) = (position.m31, position.m32);

        if self.first_char_of_word {
            let y_gap = (y - self.last_y).abs();
            if y_gap > transformed_font_size * BLOCK_BREAK_Y_FACTOR {
                self.flush_line();
                self.flush_block();
            } else if y_gap > transformed_font_size * LINE_BREAK_Y_FACTOR {
                self.flush_line();
            } else if x < self.last_end && y_gap > transformed_font_size * 0.5 {
                // moved left and down without a big enough gap to count as
                // a new block: still a wrapped line.
                self.flush_line();
            } else if x > self.last_end + transformed_font_size * 0.1 {
                self.current_line.push(' ');
            }
        }

        if self.current_block.is_empty() && self.current_line.is_empty() {
            self.block_start_font_size = Some(transformed_font_size);
        }
        self.current_line.push_str(char);

        if !char.trim().is_empty() {
            let bucket = (transformed_font_size * 2.0).round() as i64;
            *self.font_size_histogram.entry(bucket).or_insert(0) += 1;
        }

        self.first_char_of_word = false;
        self.last_y = y;
        self.last_end = x + width * transformed_font_size;
        Ok(())
    }

    fn begin_word(&mut self) -> Result<(), OutputError> {
        self.first_char_of_word = true;
        Ok(())
    }

    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }

    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

/// Compute the document's "body text" font size as the statistical mode
/// (by character count) of observed font sizes, rounded to the nearest
/// 0.5pt bucket. Body text dominates character count in essentially all
/// documents, so the mode is a robust estimator even when headings are
/// present.
fn body_text_mode_size(font_size_histogram: &HashMap<i64, usize>) -> f64 {
    font_size_histogram
        .iter()
        .max_by_key(|&(_, count)| count)
        .map(|(&bucket, _)| bucket as f64 / 2.0)
        .unwrap_or(0.0)
}

/// Bucket a heading's font size ratio (relative to body text) into a
/// heading level. This is a coarse heuristic: PDF carries no notion of
/// heading level, only relative visual size.
fn heading_level(font_size: f64, body_mode_size: f64) -> u8 {
    let ratio = if body_mode_size > 0.0 {
        font_size / body_mode_size
    } else {
        1.0
    };
    if ratio >= 1.8 {
        1
    } else if ratio >= 1.4 {
        2
    } else {
        3
    }
}

/// Collapse internal whitespace runs to single spaces and trim.
fn normalize_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

/// Create a text node with the given content.
fn text_node(content: &str) -> Node {
    Node::new(node::TEXT).prop(prop::CONTENT, content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_whitespace() {
        let text = "  Hello   world  ";
        assert_eq!(normalize_whitespace(text), "Hello world");
    }

    #[test]
    fn test_heading_level_buckets() {
        assert_eq!(heading_level(36.0, 12.0), 1); // 3.0x
        assert_eq!(heading_level(18.0, 12.0), 2); // 1.5x
        assert_eq!(heading_level(14.0, 12.0), 3); // 1.16x
    }

    #[test]
    fn test_body_text_mode_size_prefers_majority() {
        let mut histogram = HashMap::new();
        histogram.insert(24, 500); // 12.0pt bucket, 500 characters
        histogram.insert(48, 7); // 24.0pt bucket, 7 characters
        assert_eq!(body_text_mode_size(&histogram), 12.0);
    }

    #[test]
    fn test_paragraph_fixture_still_parses() {
        // Basic smoke test against the existing plain-paragraph fixture,
        // since this module no longer goes through
        // `extract_text_from_mem_by_pages`.
        let input = include_bytes!("../../../../fixtures/pdf/paragraph/input.pdf");
        let result = parse(input).expect("parse should succeed");
        assert!(!result.value.content.children.is_empty());
    }
}
