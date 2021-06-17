#![feature(test)]
extern crate test;
use scoop_hash::Sha512;

#[bench]
fn compute_0000010(bencher: &mut test::Bencher) {
    compute(10, bencher);
}
#[bench]
fn compute_0000100(bencher: &mut test::Bencher) {
    compute(100, bencher);
}
#[bench]
fn compute_0001000(bencher: &mut test::Bencher) {
    compute(1000, bencher);
}
#[bench]
fn compute_0010000(bencher: &mut test::Bencher) {
    compute(10000, bencher);
}
#[bench]
fn compute_0100000(bencher: &mut test::Bencher) {
    compute(100000, bencher);
}
#[bench]
fn compute_1000000(bencher: &mut test::Bencher) {
    compute(1000000, bencher);
}

fn compute(size: usize, bencher: &mut test::Bencher) {
    let data = &vec![0xffu8; size][..];
    bencher.iter(|| {
        test::black_box(Sha512::new().consume(data).result());
    });
    bencher.bytes = size as u64;
}
