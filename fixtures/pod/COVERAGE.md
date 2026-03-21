# POD (Plain Old Documentation) Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Reference: perlpod(1) — https://perldoc.perl.org/perlpod

## Block constructs (=command paragraphs)

- [x] paragraph (plain text block) — `paragraph`
- [x] heading level 1 (=head1) — `heading`
- [x] heading level 2 (=head2) — `heading-h2`
- [ ] heading level 3 (=head3) — (missing)
- [ ] heading level 4 (=head4) — (missing)
- [x] verbatim / code block (indented paragraph) — `code-block`
- [x] unordered list (=over / =item *) — `list-unordered`
- [x] ordered list (=over / =item 1.) — `list-ordered`
- [x] definition list (=over / =item Term) — `rare-definition-list`
- [ ] nested list — (missing)
- [ ] =begin / =end region (format block) — (missing)
- [ ] =for paragraph (format hint) — (missing)
- [ ] =encoding declaration — (missing)
- [ ] =cut (end of POD) — (missing; implicitly tested by all fixtures)
- [ ] =pod (begin of POD) — (missing)

## Inline formatting codes

- [x] B<> (bold) — `bold`
- [x] I<> (italic) — `italic`
- [x] C<> (code / monospace) — `code-inline`
- [x] U<> (underline) — `underline`
- [x] L<> (hyperlink) — `link`
- [x] E<> entity / escape (E<lt>, E<gt>, E<amp>, E<0x263A>, etc.) — `rare-formatting-codes`
- [ ] F<> (filename) — (missing)
- [ ] S<> (non-breaking spaces) — (missing)
- [ ] X<> (index entry) — (missing)
- [ ] Z<> (zero-width) — (missing)
- [ ] nested formatting codes (B<I<...>>) — (missing)
- [ ] doubled angle brackets (B<< text >>) — (missing)

## Link types (L<>)

- [x] L<URL> (bare URL) — `link`
- [ ] L<text|URL> — (missing)
- [ ] L<manpage> (man page reference) — (missing)
- [ ] L<manpage/section> — (missing)
- [ ] L<"section"> (internal section link) — (missing)
- [ ] L<text|"section"> — (missing)

## Properties

- [ ] heading level (1–4) — (heading/heading-h2 fixtures cover levels 1 and 2)
- [ ] link target URL — (missing dedicated property fixture)
- [ ] encoding from =encoding — (missing)

## Composition (integration)

- [ ] bold inside list item — (missing)
- [ ] code inside paragraph — (missing)
- [ ] link inside bold — (missing)
- [ ] definition list with inline markup in description — (missing)
- [ ] nested lists — (missing)

## Adversarial

- [x] empty document — `adv-empty`
- [ ] unclosed formatting code (B< with no >) — (missing)
- [ ] unknown formatting code letter — (missing)
- [ ] =over without =back — (missing)
- [ ] =back without =over — (missing)
- [ ] =item outside =over — (missing)
- [ ] malformed =begin / =end — (missing)

## Pathological

- [ ] very long paragraph (>64 KB) — (missing)
- [ ] deeply nested lists — (missing)
- [ ] large number of items in a list — (missing)
- [ ] deeply nested formatting codes — (missing)
