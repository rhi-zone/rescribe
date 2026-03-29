use criterion::{Criterion, criterion_group, criterion_main};
use fountain_fmt::{build, parse};

const SMALL: &str = r#"
INT. COFFEE SHOP - DAY

A BARISTA wipes down the counter. The door opens.

JOHN
Can I get a latte?

BARISTA
Coming right up.

CUT TO:
"#;

const MEDIUM: &str = r#"
Title: The Medium Script
Credit: Written by
Author: Test Author
Draft date: 2026-01-01

# ACT ONE

## Scene One

INT. OFFICE BUILDING - LOBBY - DAY

A sleek, modern lobby with marble floors. SECURITY GUARDS stand near
the revolving door entrance. Morning light streams through floor-to-ceiling
windows.

GUARD
(into radio)
We have a visitor approaching the front desk.

JOHN
Good morning. I'm here for the board meeting.

GUARD
(checking clipboard)
Name and ID please?

JOHN
John Smith, Acme Corp.

GUARD
(handing badge)
Elevator bank C, floor forty-two. They're expecting you.

> John clips the badge to his lapel and strides to the elevators. <

CUT TO:

INT. OFFICE BUILDING - CONFERENCE ROOM - DAY

A long mahogany table surrounded by leather chairs. Floor-to-ceiling windows
show the city skyline. EXECUTIVES sit around the table reviewing documents.

EXEC #1
Let's begin. The quarterly numbers are in.

EXEC #2 ^
And they don't look good.

= The meeting turns tense as the financials are revealed.

.FLASHBACK - INT. JOHN'S APARTMENT - NIGHT

A modest apartment. John sits at his kitchen table, papers spread everywhere.
He rubs his eyes and takes a sip of cold coffee.

JOHN (V.O.)
I should have seen it coming. The signs were all there.

[[Note: Consider adding a montage of warning signs here.]]

/* Director's note: We may want to reshoot this scene with different lighting
   to emphasize the isolation. Also consider adding a ticking clock sound
   effect for tension. */

~And the walls came tumbling down
~Tumbling down, tumbling down

MARY (O.S.)
John? Are you still up?

JOHN
(startled)
Just finishing some work.

MARY
It's three in the morning.

===

# ACT TWO

## Scene Five

EXT. CITY STREET - NIGHT

Rain pours down on empty streets. A single CAB idles at the curb, its
headlights cutting through the downpour. John emerges from the building,
collar turned up against the rain.

CABBIE
Where to, pal?

JOHN
Airport. International terminal.

CABBIE
Running away from something?

JOHN
(beat)
Running toward something.

> SMASH CUT TO:

INT. AIRPORT - TERMINAL - CONTINUOUS

Fluorescent lights buzz overhead. John weaves through crowds of travelers.

TICKET AGENT
One way or round trip?

JOHN
One way.

@McCLANE
(from behind)
Going somewhere, Johnny?

!John freezes. He slowly turns around.

### Beat Three

The confrontation that changes everything.

FADE OUT.

> FADE TO BLACK <
"#;

fn bench_fountain_parse_small(c: &mut Criterion) {
    c.bench_function("fountain_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_fountain_parse_medium(c: &mut Criterion) {
    c.bench_function("fountain_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_fountain_emit_medium(c: &mut Criterion) {
    c.bench_function("fountain_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_fountain_parse_small,
    bench_fountain_parse_medium,
    bench_fountain_emit_medium,
);
criterion_main!(benches);
