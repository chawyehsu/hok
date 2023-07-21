use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::Sha1;
use sha1::Digest;

fn scoop_hash_sha1(size: usize) {
    let data = &vec![0xffu8; size][..];
    Sha1::new().consume(data).result();
}

fn sha1_smol_sha1(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(data);
    hasher.digest();
}

fn sha1_sha1(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = sha1::Sha1::new();
    hasher.update(data);
    hasher.finalize();
}

fn benchmark_sha1_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_100");
    group.bench_function("scoop_hash", |b| b.iter(|| scoop_hash_sha1(black_box(100))));
    group.bench_function("sha1_smol", |b| b.iter(|| sha1_smol_sha1(black_box(100))));
    group.bench_function("sha1", |b| b.iter(|| sha1_sha1(black_box(100))));
    group.finish();
}

fn benchmark_sha1_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_1000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha1(black_box(1000)))
    });
    group.bench_function("sha1_smol", |b| b.iter(|| sha1_smol_sha1(black_box(1000))));
    group.bench_function("sha1", |b| b.iter(|| sha1_sha1(black_box(1000))));
    group.finish();
}

fn benchmark_sha1_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_10000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha1(black_box(10000)))
    });
    group.bench_function("sha1_smol", |b| b.iter(|| sha1_smol_sha1(black_box(10000))));
    group.bench_function("sha1", |b| b.iter(|| sha1_sha1(black_box(10000))));
    group.finish();
}

fn benchmark_sha1_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha1_100000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_sha1(black_box(100000)))
    });
    group.bench_function("sha1_smol", |b| {
        b.iter(|| sha1_smol_sha1(black_box(100000)))
    });
    group.bench_function("sha1", |b| b.iter(|| sha1_sha1(black_box(100000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_sha1_100,
    benchmark_sha1_1000,
    benchmark_sha1_10000,
    benchmark_sha1_100000
);
criterion_main!(benches);
