# scoop-hash

> Hashing library for [libscoop](https://docs.rs/libscoop)

[![crates-svg]][crates-url]
[![docs-svg]][docs-url]
[![crates-license-svg]][crates-url]
[![crates-download-svg]][crates-url]

[crates-svg]: https://img.shields.io/crates/v/scoop-hash.svg
[crates-url]: https://crates.io/crates/scoop-hash
[docs-svg]: https://docs.rs/scoop-hash/badge.svg
[docs-url]: https://docs.rs/scoop-hash
[crates-license-svg]: https://img.shields.io/crates/l/scoop-hash
[crates-download-svg]: https://img.shields.io/crates/d/scoop-hash.svg

This crate provides a set of hash functions used by libscoop. It is not intended
to be used by other crates.

## Install

Please refer to the repository homepage for the changelog.

```toml
[dependencies]
scoop-hash = "0.1"
```

## Hash Implementations

By default, self-contained implementations of hash functions from within this
crate are used. It is possible to use the implementations from [RustCrypto]'s
crates by enabling the `rustcrypto` feature.

```toml
[dependencies]
scoop-hash = { version = "0.1", features = ["rustcrypto"] }
```

Self-contained implementations are considerably slower than those from RustCrypto's
crates, but they do not require any external dependencies and are more portable.

## Bench

```
cargo bench
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

<sub>
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
</sub>

[RustCrypto]: https://github.com/RustCrypto/hashes
