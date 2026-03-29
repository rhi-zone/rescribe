//! Pandoc oracle harness — Pandoc does NOT read Fountain, so no oracle
//! comparison is possible.  This file contains a representative sample
//! parse-no-panic test instead.

/// Parse a representative screenplay sample and verify no panic occurs.
#[test]
fn parse_sample_no_panic() {
    let sample = r#"Title: The Test
Credit: Written by
Author: A Programmer
Draft date: 2026-03-30
Contact: test@example.com

# ACT ONE

## Scene One

INT. OFFICE BUILDING - LOBBY - DAY

A sleek, modern lobby. SECURITY GUARDS stand near the entrance.

GUARD
(into radio)
We have a visitor.

JOHN
I'm here for the meeting.

GUARD
(checking clipboard)
Name?

JOHN
John Smith.

> JOHN walks toward the elevator. <

CUT TO:

INT. OFFICE BUILDING - CONFERENCE ROOM - DAY

A long table with chairs. EXECUTIVES sit around it.

EXEC #1
Let's begin.

EXEC #2 ^
Agreed.

= The meeting begins in earnest.

~We're in the money
~We're in the money

.FLASHBACK - INT. JOHN'S APARTMENT - NIGHT

John packs his briefcase nervously.

!FADE IN:

[[Note: This scene may be cut for pacing.]]

/* This is a boneyard comment.
   It spans multiple lines.
   The parser should handle it gracefully. */

@McCLANE
Yippee ki-yay.

### Beat One

JOHN (V.O.)
I never expected this.

MARY (O.S.)
Neither did I.

===

# ACT TWO

EXT. CITY STREET - NIGHT

Rain pours down. A CAB pulls up.

FADE OUT.

> FADE TO BLACK <
"#;

    let (doc, diags) = fountain_fmt::parse(sample);
    // Should not panic
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert!(!doc.blocks.is_empty(), "document should have blocks");

    // Verify metadata was parsed
    assert_eq!(
        doc.metadata.get("title").map(|s| s.as_str()),
        Some("The Test")
    );
    assert_eq!(
        doc.metadata.get("author").map(|s| s.as_str()),
        Some("A Programmer")
    );

    // Verify we can emit without panic
    let emitted = fountain_fmt::build(&doc);
    assert!(!emitted.is_empty());

    // Verify events iterator doesn't panic
    let events: Vec<_> = fountain_fmt::events(sample).collect();
    assert!(!events.is_empty());

    // Verify strip_spans doesn't panic
    let _stripped = doc.strip_spans();
}
