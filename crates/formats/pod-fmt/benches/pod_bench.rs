use criterion::{Criterion, criterion_group, criterion_main};
use pod_fmt::{parse, build};

const SMALL: &str = r#"=head1 NAME

Example - A short POD example

=head1 SYNOPSIS

    use Example;
    my $obj = Example->new();

=head1 DESCRIPTION

This is a B<short> paragraph with C<inline code> and a L<perlpod> link.

=over 4

=item * Item one

=item * Item two

=item * Item three

=back
"#;

const MEDIUM: &str = r#"=encoding UTF-8

=head1 NAME

MediumDoc - A medium-sized POD document for benchmarking

=head1 SYNOPSIS

    use MediumDoc;
    my $doc = MediumDoc->new(%options);
    $doc->process();

=head1 DESCRIPTION

This document tests the POD parser with a medium-sized input containing many
constructs. It includes B<bold>, I<italic>, C<code>, F<filename.txt>, and
L<https://example.com> links.

=head2 Formatting Codes

POD supports B<bold text>, I<italic text>, C<code spans>, F<filenames>,
S<non breaking spaces>, and even nested codes like B<I<bold italic>>.

The E<lt>E<gt> entities allow special characters in text.

=head2 Lists

Unordered list:

=over 4

=item * First item with some text

=item * Second item with I<emphasis>

=item * Third item with C<inline code>

=back

Ordered list:

=over 4

=item 1. Step one

=item 2. Step two

=item 3. Step three

=back

Definition list:

=over 4

=item Term

A short definition of the term.

=item Another term

A longer definition with more B<detail>.

=back

=head2 Verbatim Blocks

    sub hello {
        print "Hello, world!\n";
        return 42;
    }

    my @array = (1, 2, 3);
    for my $item (@array) {
        process($item);
    }

=head2 Format Blocks

=begin html

<p>This is raw HTML content.</p>

=end html

=for text This is text-only content.

=head1 SEE ALSO

L<perlpod>, L<perlpodspec>, L<Pod::Simple>

=head1 AUTHOR

Test Author E<lt>test@example.comE<gt>

=cut
"#;

fn bench_pod_parse_small(c: &mut Criterion) {
    c.bench_function("pod_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_pod_parse_medium(c: &mut Criterion) {
    c.bench_function("pod_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_pod_roundtrip_small(c: &mut Criterion) {
    c.bench_function("pod_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_pod_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("pod_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_pod_emit_medium(c: &mut Criterion) {
    c.bench_function("pod_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_pod_parse_small,
    bench_pod_parse_medium,
    bench_pod_roundtrip_small,
    bench_pod_roundtrip_medium,
    bench_pod_emit_medium,
);
criterion_main!(benches);
