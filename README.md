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

```raw
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
to the original Scoop. Here are some random benchmarks captured in the Windows
Sandbox environment on my PC (AMD Ryzen 5 2600, 32G RAM, Windows 10).

```sh
# versions:
hok/dorado 0.1.0-beta.6
scoop-search/main 1.5.0
sfsu/extras 1.14.0
# Benchmarking scoop bucket list
Benchmark 1: scoop bucket list
  Time (mean ± σ):      5.610 s ±  0.627 s    [User: 6.573 s, System: 3.520 s]
  Range (min … max):    4.784 s …  7.063 s    10 runs

Benchmark 2: hok bucket list
  Time (mean ± σ):     159.4 ms ±  28.3 ms    [User: 86.4 ms, System: 175.2 ms]
  Range (min … max):   140.0 ms … 252.1 ms    18 runs

Summary
  hok bucket list ran
   35.19 ± 7.38 times faster than scoop bucket list
# Benchmarking scoop list
Benchmark 1: scoop list
  Time (mean ± σ):      3.577 s ±  0.043 s    [User: 4.919 s, System: 2.142 s]
  Range (min … max):    3.524 s …  3.678 s    10 runs

Benchmark 2: sfsu list
  Time (mean ± σ):      58.3 ms ±  30.8 ms    [User: 18.8 ms, System: 44.2 ms]
  Range (min … max):    39.1 ms … 234.1 ms    50 runs

Benchmark 3: hok list
  Time (mean ± σ):      48.7 ms ±  53.2 ms    [User: 13.4 ms, System: 41.7 ms]
  Range (min … max):    31.8 ms … 412.4 ms    62 runs

Summary
  hok list ran
    1.20 ± 1.45 times faster than sfsu list
   73.39 ± 80.11 times faster than scoop list
# Benchmarking scoop search (sqlite_cache enabled)
Benchmark 1: scoop search google
  Time (mean ± σ):      3.771 s ±  0.031 s    [User: 5.134 s, System: 2.085 s]
  Range (min … max):    3.725 s …  3.830 s    10 runs

Benchmark 2: scoop-search google
  Time (mean ± σ):     178.5 ms ±  14.2 ms    [User: 210.8 ms, System: 850.4 ms]
  Range (min … max):   149.4 ms … 206.8 ms    17 runs

Benchmark 3: sfsu search google
  Time (mean ± σ):      73.7 ms ±  30.1 ms    [User: 49.3 ms, System: 85.0 ms]
  Range (min … max):    52.6 ms … 202.3 ms    36 runs

Benchmark 4: hok search google
  Time (mean ± σ):      73.0 ms ±  10.2 ms    [User: 44.9 ms, System: 93.4 ms]
  Range (min … max):    63.0 ms … 109.3 ms    25 runs

Summary
  hok search google ran
    1.01 ± 0.44 times faster than sfsu search google
    2.44 ± 0.39 times faster than scoop-search google
   51.63 ± 7.25 times faster than scoop search google
```

You may run the benchmarks yourself using provided benchmark scripts in the
[`scripts` directory]. Results may vary on different environments, feel free
to share yours to help us improve the project.

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
[`scripts` directory]: scripts/benchmark/README.md
