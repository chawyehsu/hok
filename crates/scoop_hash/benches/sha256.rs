use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::Sha256;
use sha2::Digest;

fn scoop_hash_sha256(size: usize) {
    let data = &vec![0xffu8; size][..];
    Sha256::new().consume(data).result();
}

fn sha2_sha256(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize();
}

fn benchmark_sha256_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_100");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha256(black_box(100)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha256(black_box(100))));
    group.finish();
}

fn benchmark_sha256_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_1000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha256(black_box(1000)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha256(black_box(1000))));
    group.finish();
}

fn benchmark_sha256_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_10000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha256(black_box(10000)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha256(black_box(10000))));
    group.finish();
}

fn benchmark_sha256_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_100000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha256(black_box(100000)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha256(black_box(100000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_sha256_100,
    benchmark_sha256_1000,
    benchmark_sha256_10000,
    benchmark_sha256_100000
);
criterion_main!(benches);
