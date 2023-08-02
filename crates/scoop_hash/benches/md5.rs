use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::ChecksumBuilder;

fn md5(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher = ChecksumBuilder::new().md5().build();
    hasher.consume(data);
    hasher.finalize();
}

fn benchmark_md5_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_100");
    group.bench_function("scoop_hash", |b| b.iter(|| md5(black_box(100))));
    group.finish();
}

fn benchmark_md5_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_1000");
    group.bench_function("scoop_hash", |b| b.iter(|| md5(black_box(1000))));
    group.finish();
}

fn benchmark_md5_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_10000");
    group.bench_function("scoop_hash", |b| b.iter(|| md5(black_box(10000))));
    group.finish();
}

fn benchmark_md5_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_100000");
    group.bench_function("scoop_hash", |b| b.iter(|| md5(black_box(100000))));
    group.finish();
}

fn benchmark_md5_1000000(c: &mut Criterion) {
    let mut group = c.benchmark_group("md5_1000000");
    group.bench_function("scoop_hash", |b| b.iter(|| md5(black_box(1000000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_md5_100,
    benchmark_md5_1000,
    benchmark_md5_10000,
    benchmark_md5_100000,
    benchmark_md5_1000000
);
criterion_main!(benches);
