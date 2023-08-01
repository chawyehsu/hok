use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scoop_hash::Checksum;

fn sha512(size: usize) {
    let data = &vec![0xffu8; size][..];
    let mut hasher =
        Checksum::new("sha512:309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f")
            .unwrap();
    hasher.consume(data);
    hasher.result();
}

fn benchmark_sha512_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_100");
    group.bench_function("scoop_hash", |b| b.iter(|| sha512(black_box(100))));
    group.finish();
}

fn benchmark_sha512_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_1000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha512(black_box(1000))));
    group.finish();
}

fn benchmark_sha512_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_10000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha512(black_box(10000))));
    group.finish();
}

fn benchmark_sha512_100000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_100000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha512(black_box(100000))));
    group.finish();
}

fn benchmark_sha512_1000000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_1000000");
    group.bench_function("scoop_hash", |b| b.iter(|| sha512(black_box(1000000))));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_sha512_100,
    benchmark_sha512_1000,
    benchmark_sha512_10000,
    benchmark_sha512_100000,
    benchmark_sha512_1000000
);
criterion_main!(benches);
