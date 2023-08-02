use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::ChecksumBuilder;

fn sha1(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = ChecksumBuilder::new().sha1().build();
    hasher.consume(data);
    hasher.finalize();
}

fn benchmark_sha1_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_100");
    group.bench_function("scoop_hash", |b| b.iter(|| sha1(black_box(100))));
    group.finish();
}

fn benchmark_sha1_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_1000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha1(black_box(1000))));
    group.finish();
}

fn benchmark_sha1_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_10000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha1(black_box(10000))));
    group.finish();
}

fn benchmark_sha1_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_100000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha1(black_box(100000))));
    group.finish();
}

fn benchmark_sha1_1000000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_1000000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha1(black_box(1000000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_sha1_100,
    benchmark_sha1_1000,
    benchmark_sha1_10000,
    benchmark_sha1_100000,
    benchmark_sha1_1000000
);
criterion_main!(benches);
