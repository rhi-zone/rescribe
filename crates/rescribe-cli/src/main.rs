//! Rescribe CLI - Universal document converter.
//!
//! All formats supported by the `rescribe` library (54 readers, 64 writers)
//! are accessible via `--from` and `--to`.  Run `rescribe formats` to list them.

use clap::{Parser, Subcommand};
use rescribe::Document;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rescribe")]
#[command(author, version, about = "Universal document converter", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a document from one format to another
    Convert {
        /// Input file (use - for stdin)
        input: PathBuf,
        /// Output file (omit or use - for stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Input format (auto-detected from file extension if omitted)
        #[arg(short, long)]
        from: Option<String>,
        /// Output format (required if writing to stdout or a file with no known extension)
        #[arg(short, long)]
        to: Option<String>,
    },
    /// List all available formats
    Formats,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Convert {
            input,
            output,
            from,
            to,
        } => convert(input, output, from, to),
        Commands::Formats => {
            list_formats();
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Format tables
// ---------------------------------------------------------------------------

/// Metadata about one format.
struct FormatInfo {
    name: &'static str,
    aliases: &'static [&'static str],
    extensions: &'static [&'static str],
    can_read: bool,
    can_write: bool,
    /// True if the reader requires raw bytes rather than UTF-8 text.
    binary_read: bool,
    /// True if the writer produces raw bytes (content type not plain text).
    _binary_write: bool,
}

const FORMATS: &[FormatInfo] = &[
    FormatInfo {
        name: "markdown",
        aliases: &[],
        extensions: &["md", "markdown"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "markdown-strict",
        aliases: &[],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "commonmark",
        aliases: &[],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "gfm",
        aliases: &["github-markdown"],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "multimarkdown",
        aliases: &["mmd"],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "markua",
        aliases: &[],
        extensions: &["markua"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "html",
        aliases: &["html4", "html5"],
        extensions: &["html", "htm"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "latex",
        aliases: &["tex"],
        extensions: &["tex", "latex"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "org",
        aliases: &["org-mode"],
        extensions: &["org"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "rst",
        aliases: &["restructuredtext"],
        extensions: &["rst"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "asciidoc",
        aliases: &["adoc"],
        extensions: &["adoc", "asciidoc"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "typst",
        aliases: &[],
        extensions: &["typ"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "djot",
        aliases: &[],
        extensions: &["dj", "djot"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "textile",
        aliases: &[],
        extensions: &["textile"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "mediawiki",
        aliases: &[],
        extensions: &["mediawiki", "wiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "creole",
        aliases: &[],
        extensions: &["creole"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "dokuwiki",
        aliases: &[],
        extensions: &["dokuwiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "vimwiki",
        aliases: &[],
        extensions: &["vimwiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "zimwiki",
        aliases: &[],
        extensions: &["zimwiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "tikiwiki",
        aliases: &[],
        extensions: &["tikiwiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "twiki",
        aliases: &[],
        extensions: &["twiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "xwiki",
        aliases: &[],
        extensions: &["xwiki"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "jira",
        aliases: &[],
        extensions: &["jira"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "confluence",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: false,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "haddock",
        aliases: &[],
        extensions: &["haddock"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "pod",
        aliases: &[],
        extensions: &["pod"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "bbcode",
        aliases: &[],
        extensions: &["bbcode"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "fountain",
        aliases: &[],
        extensions: &["fountain"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "man",
        aliases: &["groff", "nroff"],
        extensions: &["man"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "muse",
        aliases: &[],
        extensions: &["muse"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "t2t",
        aliases: &["txt2tags"],
        extensions: &["t2t"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "texinfo",
        aliases: &["texi"],
        extensions: &["texi", "texinfo"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "docbook",
        aliases: &["docbook4", "docbook5"],
        extensions: &["docbook", "dbk"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "jats",
        aliases: &[],
        extensions: &["jats"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "tei",
        aliases: &[],
        extensions: &["tei"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "fb2",
        aliases: &[],
        extensions: &["fb2"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "opml",
        aliases: &[],
        extensions: &["opml"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "bibtex",
        aliases: &[],
        extensions: &["bib"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "biblatex",
        aliases: &[],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "csl-json",
        aliases: &["csl"],
        extensions: &["csl"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "ris",
        aliases: &[],
        extensions: &["ris"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "endnotexml",
        aliases: &[],
        extensions: &["xml"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "csv",
        aliases: &[],
        extensions: &["csv"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "tsv",
        aliases: &[],
        extensions: &["tsv"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "ipynb",
        aliases: &["jupyter"],
        extensions: &["ipynb"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "pandoc-json",
        aliases: &["json"],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "native",
        aliases: &[],
        extensions: &[],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "ansi",
        aliases: &[],
        extensions: &["ansi"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "plaintext",
        aliases: &["plain", "txt"],
        extensions: &["txt"],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "pdf",
        aliases: &[],
        extensions: &["pdf"],
        can_read: true,
        can_write: false,
        binary_read: true,
        _binary_write: true,
    },
    FormatInfo {
        name: "docx",
        aliases: &["word"],
        extensions: &["docx"],
        can_read: true,
        can_write: true,
        binary_read: true,
        _binary_write: true,
    },
    FormatInfo {
        name: "xlsx",
        aliases: &["excel"],
        extensions: &["xlsx"],
        can_read: true,
        can_write: true,
        binary_read: true,
        _binary_write: true,
    },
    FormatInfo {
        name: "epub",
        aliases: &[],
        extensions: &["epub"],
        can_read: true,
        can_write: true,
        binary_read: true,
        _binary_write: true,
    },
    FormatInfo {
        name: "odt",
        aliases: &[],
        extensions: &["odt"],
        can_read: true,
        can_write: true,
        binary_read: true,
        _binary_write: true,
    },
    FormatInfo {
        name: "pptx",
        aliases: &["powerpoint"],
        extensions: &["pptx"],
        can_read: true,
        can_write: true,
        binary_read: true,
        _binary_write: true,
    },
    // Writers only
    FormatInfo {
        name: "rtf",
        aliases: &[],
        extensions: &["rtf"],
        can_read: true,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "beamer",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "revealjs",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "slidy",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "slideous",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "s5",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "dzslides",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "context",
        aliases: &[],
        extensions: &["ctx"],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "ms",
        aliases: &[],
        extensions: &["ms"],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "icml",
        aliases: &[],
        extensions: &["icml"],
        can_read: false,
        can_write: true,
        binary_read: false,
        _binary_write: false,
    },
    FormatInfo {
        name: "chunkedhtml",
        aliases: &[],
        extensions: &[],
        can_read: false,
        can_write: false,
        binary_read: false,
        _binary_write: false,
    },
];

fn find_format(name: &str) -> Option<&'static FormatInfo> {
    let lower = name.to_lowercase();
    FORMATS
        .iter()
        .find(|f| f.name == lower || f.aliases.contains(&lower.as_str()))
}

fn format_from_extension(ext: &str) -> Option<&'static FormatInfo> {
    let lower = ext.to_lowercase();
    FORMATS
        .iter()
        .find(|f| f.extensions.contains(&lower.as_str()) && (f.can_read || f.can_write))
}

// ---------------------------------------------------------------------------
// Conversion entry point
// ---------------------------------------------------------------------------

fn convert(
    input: PathBuf,
    output: Option<PathBuf>,
    from: Option<String>,
    to: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine input format.
    let in_fmt = from
        .as_deref()
        .and_then(find_format)
        .or_else(|| {
            if input.as_os_str() == "-" {
                return None;
            }
            input
                .extension()
                .and_then(|e| e.to_str())
                .and_then(format_from_extension)
        })
        .ok_or_else(|| {
            if let Some(ref name) = from {
                format!(
                    "unknown format {:?}; run `rescribe formats` for a list",
                    name
                )
            } else {
                "cannot detect input format; use --from".into()
            }
        })?;

    if !in_fmt.can_read {
        return Err(format!("format {:?} has no reader", in_fmt.name).into());
    }

    // Determine output format.
    let out_fmt = to
        .as_deref()
        .and_then(find_format)
        .or_else(|| {
            output.as_ref().and_then(|p| {
                if p.as_os_str() == "-" {
                    return None;
                }
                p.extension()
                    .and_then(|e| e.to_str())
                    .and_then(format_from_extension)
            })
        })
        .ok_or_else(|| {
            if let Some(ref name) = to {
                format!(
                    "unknown format {:?}; run `rescribe formats` for a list",
                    name
                )
            } else {
                "cannot detect output format; use --to".into()
            }
        })?;

    if !out_fmt.can_write {
        return Err(format!("format {:?} has no writer", out_fmt.name).into());
    }

    // Read input.
    let doc = if in_fmt.binary_read {
        let bytes = read_bytes(&input)?;
        parse_binary(&bytes, in_fmt.name)?
    } else {
        let text = read_text(&input)?;
        parse_text(&text, in_fmt.name)?
    };

    // Emit output.
    let out_bytes = emit(&doc, out_fmt.name)?;

    // Write output.
    match output {
        Some(ref p) if p.as_os_str() != "-" => fs::write(p, &out_bytes)?,
        _ => io::stdout().write_all(&out_bytes)?,
    }

    Ok(())
}

fn read_bytes(path: &PathBuf) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if path.as_os_str() == "-" {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        Ok(fs::read(path)?)
    }
}

fn read_text(path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        Ok(fs::read_to_string(path)?)
    }
}

// ---------------------------------------------------------------------------
// Readers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn parse_text(input: &str, fmt: &str) -> Result<Document, Box<dyn std::error::Error>> {
    macro_rules! r {
        ($mod:ident) => {{
            let res = rescribe::$mod::parse(input)?;
            for w in &res.warnings {
                eprintln!("warning: {}", w.message);
            }
            return Ok(res.value);
        }};
    }
    match fmt {
        "markdown" => r!(markdown),
        "markdown-strict" => r!(markdown_strict),
        "commonmark" => r!(commonmark),
        "gfm" => r!(gfm),
        "multimarkdown" => r!(multimarkdown),
        "markua" => r!(markua),
        "html" => r!(html),
        "latex" => r!(latex),
        "org" => r!(org),
        "rst" => r!(rst),
        "asciidoc" => r!(asciidoc),
        "typst" => r!(typst),
        "djot" => r!(djot),
        "textile" => r!(textile),
        "mediawiki" => r!(mediawiki),
        "creole" => r!(creole),
        "dokuwiki" => r!(dokuwiki),
        "vimwiki" => r!(vimwiki),
        "zimwiki" => r!(zimwiki),
        "tikiwiki" => r!(tikiwiki),
        "twiki" => r!(twiki),
        "xwiki" => r!(xwiki),
        "jira" => r!(jira),
        "haddock" => r!(haddock),
        "pod" => r!(pod),
        "bbcode" => r!(bbcode),
        "fountain" => r!(fountain),
        "man" => r!(man),
        "muse" => r!(muse),
        "t2t" => r!(t2t),
        "texinfo" => r!(texinfo),
        "docbook" => r!(docbook),
        "jats" => r!(jats),
        "tei" => r!(tei),
        "fb2" => r!(fb2),
        "opml" => r!(opml),
        "bibtex" => r!(bibtex),
        "biblatex" => r!(biblatex),
        "csl-json" => r!(csl_json),
        "ris" => r!(ris),
        "endnotexml" => r!(endnotexml),
        "csv" => r!(csv),
        "tsv" => r!(tsv),
        "ipynb" => r!(ipynb),
        "pandoc-json" => {
            let res = rescribe::pandoc_json::parse(input)?;
            for w in &res.warnings {
                eprintln!("warning: {}", w.message);
            }
            Ok(res.value)
        }
        "native" => r!(native),
        "ansi" => r!(ansi),
        "rtf" => r!(rtf),
        _ => Err(format!("no text reader for {fmt:?}").into()),
    }
}

fn parse_binary(input: &[u8], fmt: &str) -> Result<Document, Box<dyn std::error::Error>> {
    macro_rules! rb {
        ($mod:ident, $fn:ident) => {{
            let res = rescribe::$mod::$fn(input)?;
            for w in &res.warnings {
                eprintln!("warning: {}", w.message);
            }
            return Ok(res.value);
        }};
    }
    match fmt {
        "pdf" => rb!(pdf, parse),
        "docx" => rb!(docx, parse_bytes),
        "xlsx" => rb!(xlsx, parse_bytes),
        "epub" => rb!(epub, parse_bytes),
        "odt" => rb!(odt, parse),
        "pptx" => rb!(pptx, parse),
        _ => Err(format!("no binary reader for {fmt:?}").into()),
    }
}

// ---------------------------------------------------------------------------
// Writers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn emit(doc: &Document, fmt: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    macro_rules! w {
        ($mod:ident) => {{
            let res = rescribe::$mod::emit(doc)?;
            for w in &res.warnings {
                eprintln!("warning: {}", w.message);
            }
            return Ok(res.value);
        }};
    }
    match fmt {
        "markdown"        => w!(markdown),
        "markdown-strict" => w!(markdown_strict),
        "commonmark"      => w!(commonmark),
        "gfm"             => w!(gfm),
        "multimarkdown"   => w!(multimarkdown),
        "markua"          => w!(markua),
        "html"            => w!(html),
        "latex"           => w!(latex),
        "org"             => w!(org),
        "rst"             => w!(rst),
        "asciidoc"        => w!(asciidoc),
        "typst"           => w!(typst),
        "djot"            => w!(djot),
        "textile"         => w!(textile),
        "mediawiki"       => w!(mediawiki),
        "creole"          => w!(creole),
        "dokuwiki"        => w!(dokuwiki),
        "vimwiki"         => w!(vimwiki),
        "zimwiki"         => w!(zimwiki),
        "tikiwiki"        => w!(tikiwiki),
        "twiki"           => w!(twiki),
        "xwiki"           => w!(xwiki),
        "jira"            => w!(jira),
        "haddock"         => w!(haddock),
        "pod"             => w!(pod),
        "bbcode"          => w!(bbcode),
        "fountain"        => w!(fountain),
        "man"             => w!(man),
        "muse"            => w!(muse),
        "t2t"             => w!(t2t),
        "texinfo"         => w!(texinfo),
        "docbook"         => w!(docbook),
        "jats"            => w!(jats),
        "tei"             => w!(tei),
        "fb2"             => w!(fb2),
        "opml"            => w!(opml),
        "bibtex"          => w!(bibtex),
        "biblatex"        => w!(biblatex),
        "csl-json"        => w!(csl_json),
        "ris"             => w!(ris),
        "endnotexml"      => w!(endnotexml),
        "csv"             => w!(csv),
        "tsv"             => w!(tsv),
        "ipynb"           => w!(ipynb),
        "pandoc-json"     => { let res = rescribe::pandoc_json::emit(doc)?; for w in &res.warnings { eprintln!("warning: {}", w.message); } Ok(res.value) }
        "native"          => w!(native),
        "ansi"            => w!(ansi),
        "plaintext"       => w!(plaintext),
        "rtf"             => w!(rtf),
        "beamer"          => w!(beamer),
        "revealjs"        => w!(revealjs),
        "slidy"           => w!(slidy),
        "slideous"        => w!(slideous),
        "s5"              => w!(s5),
        "dzslides"        => w!(dzslides),
        "context"         => w!(context),
        "ms"              => w!(ms),
        "icml"            => w!(icml),
        "chunkedhtml"     => Err("chunkedhtml produces multiple files and cannot be used as a single-output writer via CLI".into()),
        "docx"            => { let res = rescribe::docx::emit(doc)?; for w in &res.warnings { eprintln!("warning: {}", w.message); } Ok(res.value) }
        "xlsx"            => { let res = rescribe::xlsx::emit(doc)?; for w in &res.warnings { eprintln!("warning: {}", w.message); } Ok(res.value) }
        "epub"            => { let res = rescribe::epub::emit(doc)?; for w in &res.warnings { eprintln!("warning: {}", w.message); } Ok(res.value) }
        "odt"             => { let res = rescribe::odt::emit(doc)?;  for w in &res.warnings { eprintln!("warning: {}", w.message); } Ok(res.value) }
        "pptx"            => { let res = rescribe::pptx::emit(doc)?; for w in &res.warnings { eprintln!("warning: {}", w.message); } Ok(res.value) }
        _ => Err(format!("no writer for {fmt:?}").into()),
    }
}

// ---------------------------------------------------------------------------
// Format listing
// ---------------------------------------------------------------------------

fn list_formats() {
    println!(
        "Available formats ({} total):\n",
        FORMATS.iter().filter(|f| f.can_read || f.can_write).count()
    );
    println!(
        "  {:<20} {:<5} {:<5}  EXTENSIONS",
        "FORMAT", "READ", "WRITE"
    );
    println!(
        "  {:<20} {:<5} {:<5}  ----------",
        "------", "----", "-----"
    );
    for f in FORMATS {
        if !f.can_read && !f.can_write {
            continue;
        }
        let r = if f.can_read { "yes" } else { "-" };
        let w = if f.can_write { "yes" } else { "-" };
        let exts = f.extensions.join(", ");
        println!("  {:<20} {:<5} {:<5}  {}", f.name, r, w, exts);
    }
}
