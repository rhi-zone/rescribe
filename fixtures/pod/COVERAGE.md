# POD (Plain Old Documentation) Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Reference: perlpod(1) — https://perldoc.perl.org/perlpod

## Block constructs (=command paragraphs)

- [x] paragraph (plain text block) — `paragraph`
- [x] heading level 1 (=head1) — `heading`
- [x] heading level 2 (=head2) — `heading-h2`
- [x] heading level 3 (=head3) — `heading-h3`
- [x] heading level 4 (=head4) — `heading-h4`
- [x] verbatim / code block (indented paragraph) — `code-block`
- [x] unordered list (=over / =item *) — `list-unordered`
- [x] ordered list (=over / =item 1.) — `list-ordered`
- [x] definition list (=over / =item Term) — `rare-definition-list`
- [x] nested list — `nested-list`
- [x] =begin / =end region (format block) — `begin-end`
- [x] =for paragraph (format hint) — `for-block`
- [x] =encoding declaration — `encoding`
- [x] =cut (end of POD) — `pod-cut`
- [x] =pod (begin of POD) — `pod-cut`

## Inline formatting codes

- [x] B<> (bold) — `bold`
- [x] I<> (italic) — `italic`
- [x] C<> (code / monospace) — `code-inline`
- [x] U<> (underline) — `underline`
- [x] L<> (hyperlink) — `link`
- [x] E<> entity / escape (E<lt>, E<gt>, E<amp>, E<0x263A>, etc.) — `rare-formatting-codes`
- [x] F<> (filename) — `filename`
- [x] S<> (non-breaking spaces) — `non-breaking`
- [x] X<> (index entry) — `index-entry`
- [x] Z<> (zero-width) — `zero-width`
- [x] nested formatting codes (B<I<...>>) — `nested-formatting`
- [x] doubled angle brackets (B<< text >>) — `double-brackets`

## Link types (L<>)

- [x] L<URL> (bare URL) — `link`
- [x] L<text|URL> — `link-with-label`
- [x] L<manpage> (man page reference) — `link-manpage`
- [x] L<manpage/section> — `link-manpage-section`
- [x] L<"section"> (internal section link) — `link-section`
- [x] L<text|"section"> — `link-text-section`

## Properties

- [x] heading level (1–4) — `heading-levels`
- [x] link target URL — `link-url-property`
- [x] encoding from =encoding — `encoding-property`

## Composition (integration)

- [x] bold inside list item — `comp-bold-in-list`
- [x] code inside paragraph — `comp-code-in-para`
- [x] link inside bold — `comp-link-in-bold`
- [x] definition list with inline markup in description — `comp-deflist-markup`
- [x] nested lists — `comp-nested-lists`

## Adversarial

- [x] empty document — `adv-empty`
- [x] unclosed formatting code (B< with no >) — `adv-unclosed-format`
- [x] unknown formatting code letter — `adv-unknown-format-code`
- [x] =over without =back — `adv-over-no-back`
- [x] =back without =over — `adv-back-no-over`
- [x] =item outside =over — `adv-item-no-over`
- [x] malformed =begin / =end — `adv-malformed-begin-end`

## Pathological

- [x] very long paragraph (>64 KB) — `path-long-para`
- [x] deeply nested lists — `path-deep-nested-lists`
- [x] large number of items in a list — `path-many-items`
- [x] deeply nested formatting codes — `path-deep-nested-formatting`
