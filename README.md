# hok

> Hok is a CLI implementation of [Scoop](https://scoop.sh/) in Rust

[![cicd][cicd-badge]][cicd]
[![release][release-badge]][releases]
[![crates-svg]][crates-url]
[![license][license-badge]](LICENSE)
[![downloads][downloads-badge]][releases]
[![docs-svg]][docs-url]

[简体中文]

## Install

🚧 **CAVEAT**: Under heavy development, interfaces may change without notice.

Assuming you have the original Scoop installed, simply run:

```sh
scoop bucket add dorado https://github.com/chawyehsu/dorado
scoop install dorado/hok
```

Note this will add the `dorado` bucket I maintain to install Hok. Hok is experimental and it's currently only available in that bucket.

## Commands

The command line interface is similar to Scoop.

```
$ hok help
Hok is a CLI implementation of Scoop in Rust

Usage: hok.exe <COMMAND>

Commands:
  bucket     Manage manifest buckets
  cache      Package cache management
  cat        Inspect the manifest of a package
  cleanup    Cleanup apps by removing old versions
  config     Configuration management
  hold       Hold package(s) to disable changes
  home       Browse the homepage of a package
  info       Show package(s) basic information
  install    Install package(s)
  list       List installed package(s)
  search     Search available package(s)
  unhold     Unhold package(s) to enable changes
  uninstall  Uninstall package(s)
  update     Fetch and update subscribed buckets
  upgrade    Upgrade installed package(s)
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Type 'hok help <command>' to get help for a specific command.
```

## Development

Prerequisites: Git, Rust

```sh
# clone the repo
git clone https://github.com/chawyehsu/hok
cd hok
# build
cargo build
# run and test
cargo run -- help
```

## Performance

Hok (also the libscoop backend) aims to provide a faster yet powerful alternative
to the original Scoop. Here are some random benchmarks tested with v0.1.0-beta.2
in a Windows Sandbox environment on my PC (AMD Ryzen 5 2600, Windows 10).

```
hok version:
hok 0.1.0-beta.2

Benchmarking scoop bucket list
Benchmark 1: scoop bucket list
  Time (mean ± σ):      5.030 s ±  0.269 s    [User: 5.785 s, System: 3.101 s]
  Range (min … max):    4.751 s …  5.676 s    10 runs

Benchmark 2: hok bucket list
  Time (mean ± σ):     142.0 ms ±  18.1 ms    [User: 156.1 ms, System: 188.5 ms]
  Range (min … max):   123.4 ms … 190.7 ms    21 runs

Summary
  hok bucket list ran
   35.43 ± 4.91 times faster than scoop bucket list

Benchmarking scoop list
Benchmark 1: scoop list
  Time (mean ± σ):      3.472 s ±  0.134 s    [User: 4.990 s, System: 2.005 s]
  Range (min … max):    3.285 s …  3.660 s    10 runs

Benchmark 2: hok list
  Time (mean ± σ):      47.3 ms ±  31.6 ms    [User: 14.7 ms, System: 39.7 ms]
  Range (min … max):    33.1 ms … 256.0 ms    68 runs

Summary
  hok list ran
   73.42 ± 49.15 times faster than scoop list

Benchmarking scoop search
Benchmark 1: scoop search google
  Time (mean ± σ):     20.688 s ±  0.373 s    [User: 17.764 s, System: 7.032 s]
  Range (min … max):   20.279 s … 21.625 s    10 runs

Benchmark 2: scoop-search google
  Time (mean ± σ):     258.4 ms ±  31.2 ms    [User: 168.8 ms, System: 563.4 ms]
  Range (min … max):   223.8 ms … 305.1 ms    10 runs

Benchmark 3: hok search google
  Time (mean ± σ):      69.0 ms ±  22.9 ms    [User: 71.3 ms, System: 87.9 ms]
  Range (min … max):    44.3 ms … 197.3 ms    44 runs

Summary
  hok search google ran
    3.75 ± 1.33 times faster than scoop-search google
  299.91 ± 99.87 times faster than scoop search google
```

You may run the benchmarks yourself using provided benchmark scripts in the
`scripts` directory, feel free to share your results.

## Roadmap

TBD

## License

**hok** © [Chawye Hsu](https://github.com/chawyehsu). Released under the [Apache-2.0](LICENSE) license.
For licenses of sub crates, see [COPYING](COPYING).

> [Blog](https://chawyehsu.com) · GitHub [@chawyehsu](https://github.com/chawyehsu) · Twitter [@chawyehsu](https://twitter.com/chawyehsu)

[cicd-badge]: https://github.com/chawyehsu/hok/workflows/CICD/badge.svg
[cicd]: https://github.com/chawyehsu/hok/actions/workflows/cicd.yml
[release-badge]: https://img.shields.io/github/v/release/chawyehsu/hok
[releases]: https://github.com/chawyehsu/hok/releases/latest
[crates-svg]: https://img.shields.io/crates/v/libscoop.svg
[crates-url]: https://crates.io/crates/libscoop
[license-badge]: https://img.shields.io/github/license/chawyehsu/hok
[downloads-badge]: https://img.shields.io/github/downloads/chawyehsu/hok/total
[docs-svg]: https://docs.rs/libscoop/badge.svg
[docs-url]: https://docs.rs/libscoop
[简体中文]: https://chawyehsu.com/blog/reimplementing-scoop-in-rust
