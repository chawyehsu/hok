use criterion::{black_box, criterion_group, criterion_main, Criterion};
use md_5::Digest;
use scoop_hash::Md5;

fn scoop_hash_md5(size: usize) {
    let data = &vec![0xffu8; size][..];
    Md5::new().consume(data).result();
}

fn md5_md5(size: usize) {
    let data = &vec![0xffu8; size][..];
    md5::compute(data);
}

fn md_5_md5(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = md_5::Md5::new();
    hasher.update(data);
    hasher.finalize();
}

fn benchmark_md5_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_100");
    group.bench_function("scoop_hash", |b| b.iter(|| scoop_hash_md5(black_box(100))));
    group.bench_function("md5", |b| b.iter(|| md5_md5(black_box(100))));
    group.bench_function("md_5", |b| b.iter(|| md_5_md5(black_box(100))));
    group.finish();
}

fn benchmark_md5_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_1000");
    group.bench_function("scoop_hash", |b| b.iter(|| scoop_hash_md5(black_box(1000))));
    group.bench_function("md5", |b| b.iter(|| md5_md5(black_box(1000))));
    group.bench_function("md_5", |b| b.iter(|| md_5_md5(black_box(1000))));
    group.finish();
}

fn benchmark_md5_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_10000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_md5(black_box(10000)))
    });
    group.bench_function("md5", |b| b.iter(|| md5_md5(black_box(10000))));
    group.bench_function("md_5", |b| b.iter(|| md_5_md5(black_box(10000))));
    group.finish();
}

fn benchmark_md5_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_100000");
    group.bench_function("scoop_hash", |b| {
        b.iter(|| scoop_hash_md5(black_box(100000)))
    });
    group.bench_function("md5", |b| b.iter(|| md5_md5(black_box(100000))));
    group.bench_function("md_5", |b| b.iter(|| md_5_md5(black_box(100000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_md5_100,
    benchmark_md5_1000,
    benchmark_md5_10000,
    benchmark_md5_100000
);
criterion_main!(benches);
