use criterion::{Criterion, criterion_group, criterion_main};
use rst_fmt::{parse, build};

const SMALL: &str = r#"
Hello World
===========

This is a **short** paragraph with `inline code` and a `link <https://example.com>`_.

- Item one
- Item two
- Item three

.. code-block:: rust

   fn main() {
       println!("hello");
   }
"#;

const MEDIUM: &str = r#"
Document Title
==============

Introduction
------------

This document tests the RST parser with a medium-sized input containing many constructs.
It includes **bold**, *italic*, ``code``, and `hyperlinks <https://example.com>`_.

Lists
-----

Unordered list:

- First item with some text
- Second item with *emphasis*
- Third item with ``inline code``

Ordered list:

1. Step one
2. Step two
3. Step three

Definition lists:

term
    A short definition of the term.

another term
    A longer definition with more detail.

Tables
------

+----------+----------+----------+
| Header 1 | Header 2 | Header 3 |
+==========+==========+==========+
| Cell A   | Cell B   | Cell C   |
+----------+----------+----------+
| Cell D   | Cell E   | Cell F   |
+----------+----------+----------+

Code Blocks
-----------

.. code-block:: python

   def hello():
       print("Hello, world!")
       return 42

.. code-block:: rust

   pub fn add(a: i32, b: i32) -> i32 {
       a + b
   }

Blockquotes
-----------

    This is a blockquote that spans
    multiple lines of text.

Footnotes
---------

This text has a footnote [#]_.

.. [#] This is the footnote text.

Images
------

.. image:: /path/to/image.png
   :alt: An example image
   :width: 400

Directives
----------

.. note::

   This is a note directive.

.. warning::

   This is a warning directive.

Line Blocks
-----------

| Line one
| Line two
| Line three

Field Lists
-----------

:Author: Jane Doe
:Date: 2024-01-01
:Version: 1.0

More Content
------------

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.
"#;

fn bench_rst_parse_small(c: &mut Criterion) {
    c.bench_function("rst_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_rst_parse_medium(c: &mut Criterion) {
    c.bench_function("rst_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_rst_roundtrip_small(c: &mut Criterion) {
    c.bench_function("rst_roundtrip_small", |b| {
        let doc = parse(SMALL).expect("parse ok");
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_rst_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("rst_roundtrip_medium", |b| {
        let doc = parse(MEDIUM).expect("parse ok");
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_rst_emit_medium(c: &mut Criterion) {
    c.bench_function("rst_emit_medium", |b| {
        let doc = parse(MEDIUM).expect("parse ok");
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_rst_parse_small,
    bench_rst_parse_medium,
    bench_rst_roundtrip_small,
    bench_rst_roundtrip_medium,
    bench_rst_emit_medium,
);
criterion_main!(benches);
