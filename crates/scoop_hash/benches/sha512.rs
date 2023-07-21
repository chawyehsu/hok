use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::Sha512;
use sha2::Digest;

fn scoop_hash_sha512(size: usize) {
    let data = &vec![0xffu8; size][..];
    Sha512::new().consume(data).result();
}

fn sha2_sha512(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    hasher.finalize();
}

fn benchmark_sha512_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_100");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha512(black_box(100)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha512(black_box(100))));
    group.finish();
}

fn benchmark_sha512_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_1000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha512(black_box(1000)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha512(black_box(1000))));
    group.finish();
}

fn benchmark_sha512_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_10000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha512(black_box(10000)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha512(black_box(10000))));
    group.finish();
}

fn benchmark_sha512_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_100000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha512(black_box(100000)))
    });
    group.bench_function("sha2", |b| b.iter(|| sha2_sha512(black_box(100000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_sha512_100,
    benchmark_sha512_1000,
    benchmark_sha512_10000,
    benchmark_sha512_100000
);
criterion_main!(benches);
