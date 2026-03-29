use criterion::{Criterion, criterion_group, criterion_main};
use man_fmt::{build, parse};

const SMALL: &str = r#".TH TEST 1 "2024-01-01" "Test Suite" "User Commands"
.SH NAME
test \- a test program
.SH SYNOPSIS
.B test
[\fIoptions\fR]
.SH DESCRIPTION
This is a short test program for benchmarking the man page parser.
It includes \fBbold\fR and \fIitalic\fR text.
"#;

const MEDIUM: &str = r#".TH MYAPP 1 "2024-06-15" "MyApp 2.0" "User Commands"
.SH NAME
myapp \- a medium-sized application
.SH SYNOPSIS
.B myapp
[\fB\-v\fR]
[\fB\-o\fR \fIoutput\fR]
.I input
.SH DESCRIPTION
.PP
This is a more detailed description of myapp.
It spans multiple paragraphs and includes various formatting constructs.
.PP
The program supports \fBbold text\fR, \fIitalic text\fR, and
\f(CWinline code\fR formatting.
.SH OPTIONS
.TP
\fB\-v\fR
Enable verbose mode.
Print additional information during processing.
.TP
\fB\-o\fR \fIoutput\fR
Write output to the specified file instead of stdout.
.TP
\fB\-h\fR
Display help message and exit.
.SH EXAMPLES
.PP
Basic usage:
.EX
myapp input.txt
.EE
.PP
With output file:
.EX
myapp -o result.txt input.txt
.EE
.SH SEE ALSO
.BR related (1),
.BR other (5)
.SH BUGS
.PP
Report bugs to <bugs@example.com>.
.PP
Known issues:
.IP \(bu
Memory usage may be high for large inputs.
.IP \(bu
Unicode support is incomplete.
.SH AUTHORS
Written by Test Author.
.SH COPYRIGHT
Copyright \(co 2024 Test Organization.
Licensed under the MIT License.
"#;

fn bench_man_parse_small(c: &mut Criterion) {
    c.bench_function("man_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_man_parse_medium(c: &mut Criterion) {
    c.bench_function("man_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_man_emit_medium(c: &mut Criterion) {
    c.bench_function("man_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

fn bench_man_roundtrip_small(c: &mut Criterion) {
    c.bench_function("man_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_man_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("man_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

criterion_group!(
    benches,
    bench_man_parse_small,
    bench_man_parse_medium,
    bench_man_emit_medium,
    bench_man_roundtrip_small,
    bench_man_roundtrip_medium,
);
criterion_main!(benches);
