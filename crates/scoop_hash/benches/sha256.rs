use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::ChecksumBuilder;

fn sha256(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = ChecksumBuilder::new().sha256().build();
    hasher.consume(data);
    hasher.result();
}

fn benchmark_sha256_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_100");
    group.bench_function("scoop_hash", |b| b.iter(|| sha256(black_box(100))));
    group.finish();
}

fn benchmark_sha256_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_1000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha256(black_box(1000))));
    group.finish();
}

fn benchmark_sha256_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_10000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha256(black_box(10000))));
    group.finish();
}

fn benchmark_sha256_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_100000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha256(black_box(100000))));
    group.finish();
}

fn benchmark_sha256_1000000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_1000000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha256(black_box(1000000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_sha256_100,
    benchmark_sha256_1000,
    benchmark_sha256_10000,
    benchmark_sha256_100000,
    benchmark_sha256_1000000
);
criterion_main!(benches);
