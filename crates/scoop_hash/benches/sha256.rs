use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::Checksum;

fn sha256(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher =
        Checksum::new("sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
            .unwrap();
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
