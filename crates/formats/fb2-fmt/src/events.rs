//! SAX-style event iterator over a parsed FictionBook AST.

use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    StartFictionBook,
    EndFictionBook,
    Metadata(&'a FictionBook),
    StartBody { name: Option<&'a str>, lang: Option<&'a str> },
    EndBody,
    StartSection { id: Option<&'a str>, lang: Option<&'a str> },
    EndSection,
    StartTitle,
    EndTitle,
    TitleParagraph(Vec<InlineElement>),
    StartParagraph,
    EndParagraph,
    Inline(Vec<InlineElement>),
    StartPoem,
    EndPoem,
    StartStanza,
    EndStanza,
    VerseLine(Vec<InlineElement>),
    EmptyLine,
    Subtitle(Vec<InlineElement>),
    StartCite { id: Option<&'a str> },
    EndCite,
    StartEpigraph { id: Option<&'a str> },
    EndEpigraph,
    TextAuthor(Vec<InlineElement>),
    Binary(&'a Binary),
}

/// Walks the FictionBook AST and yields events.
pub struct EventIter<'a> {
    events: std::vec::IntoIter<Event<'a>>,
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.events.next()
    }
}

/// Walk a parsed `FictionBook` and emit SAX-style events.
pub fn events(fb: &FictionBook) -> EventIter<'_> {
    let mut out: Vec<Event<'_>> = Vec::new();
    collect_events(fb, &mut out);
    EventIter { events: out.into_iter() }
}

fn collect_events<'a>(fb: &'a FictionBook, out: &mut Vec<Event<'a>>) {
    out.push(Event::StartFictionBook);
    out.push(Event::Metadata(fb));

    for body in &fb.bodies {
        out.push(Event::StartBody {
            name: body.name.as_deref(),
            lang: body.lang.as_deref(),
        });
        if let Some(title) = &body.title {
            collect_title_events(title, out);
        }
        for epigraph in &body.epigraph {
            collect_epigraph_events(epigraph, out);
        }
        for section in &body.section {
            collect_section_events(section, out);
        }
        out.push(Event::EndBody);
    }

    for binary in &fb.binaries {
        out.push(Event::Binary(binary));
    }

    out.push(Event::EndFictionBook);
}

fn collect_section_events<'a>(section: &'a Section, out: &mut Vec<Event<'a>>) {
    out.push(Event::StartSection {
        id: section.id.as_deref(),
        lang: section.lang.as_deref(),
    });
    if let Some(title) = &section.title {
        collect_title_events(title, out);
    }
    for epigraph in &section.epigraph {
        collect_epigraph_events(epigraph, out);
    }
    for content in &section.content {
        collect_section_content_events(content, out);
    }
    for nested in &section.section {
        collect_section_events(nested, out);
    }
    out.push(Event::EndSection);
}

fn collect_title_events<'a>(title: &'a Title, out: &mut Vec<Event<'a>>) {
    out.push(Event::StartTitle);
    for para in &title.para {
        match para {
            TitlePara::Para(il) => out.push(Event::TitleParagraph(il.clone())),
            TitlePara::EmptyLine => out.push(Event::EmptyLine),
        }
    }
    out.push(Event::EndTitle);
}

fn collect_section_content_events<'a>(content: &'a SectionContent, out: &mut Vec<Event<'a>>) {
    match content {
        SectionContent::Para(il) => {
            out.push(Event::StartParagraph);
            out.push(Event::Inline(il.clone()));
            out.push(Event::EndParagraph);
        }
        SectionContent::EmptyLine => out.push(Event::EmptyLine),
        SectionContent::Subtitle(il) => out.push(Event::Subtitle(il.clone())),
        SectionContent::Image(_) => {} // skipped in event stream
        SectionContent::Poem(p) => collect_poem_events(p, out),
        SectionContent::Cite(c) => collect_cite_events(c, out),
        SectionContent::Table(_) => {} // tables skipped in basic event stream
    }
}

fn collect_poem_events<'a>(poem: &'a Poem, out: &mut Vec<Event<'a>>) {
    out.push(Event::StartPoem);
    if let Some(title) = &poem.title {
        collect_title_events(title, out);
    }
    for epigraph in &poem.epigraph {
        collect_epigraph_events(epigraph, out);
    }
    for stanza in &poem.stanza {
        out.push(Event::StartStanza);
        for v in &stanza.v {
            out.push(Event::VerseLine(v.clone()));
        }
        out.push(Event::EndStanza);
    }
    for ta in &poem.text_author {
        out.push(Event::TextAuthor(ta.clone()));
    }
    out.push(Event::EndPoem);
}

fn collect_cite_events<'a>(cite: &'a Cite, out: &mut Vec<Event<'a>>) {
    out.push(Event::StartCite { id: cite.id.as_deref() });
    for content in &cite.content {
        match content {
            CiteContent::Para(il) => {
                out.push(Event::StartParagraph);
                out.push(Event::Inline(il.clone()));
                out.push(Event::EndParagraph);
            }
            CiteContent::EmptyLine => out.push(Event::EmptyLine),
            CiteContent::Poem(p) => collect_poem_events(p, out),
            CiteContent::Table(_) => {}
        }
    }
    for ta in &cite.text_author {
        out.push(Event::TextAuthor(ta.clone()));
    }
    out.push(Event::EndCite);
}

fn collect_epigraph_events<'a>(epigraph: &'a Epigraph, out: &mut Vec<Event<'a>>) {
    out.push(Event::StartEpigraph { id: epigraph.id.as_deref() });
    for content in &epigraph.content {
        match content {
            EpigraphContent::Para(il) => {
                out.push(Event::StartParagraph);
                out.push(Event::Inline(il.clone()));
                out.push(Event::EndParagraph);
            }
            EpigraphContent::EmptyLine => out.push(Event::EmptyLine),
            EpigraphContent::Poem(p) => collect_poem_events(p, out),
            EpigraphContent::Cite(c) => collect_cite_events(c, out),
        }
    }
    for ta in &epigraph.text_author {
        out.push(Event::TextAuthor(ta.clone()));
    }
    out.push(Event::EndEpigraph);
}
