# hok

> Hok is a CLI implementation of Scoop in Rust

[![cicd][cicd-badge]][cicd]
[![release][release-badge]][releases]
[![license][license-badge]](LICENSE)
[![downloads][downloads-badge]][releases]

## Install

🚧 **CAVEAT**: Under heavy development, interfaces may change without notice.

Assuming you have the original Scoop installed, simply run:

```sh
scoop bucket add dorado https://github.com/chawyehsu/dorado
scoop install dorado/hok
```

## Commands

The command line interface is similar to Scoop.

```
$ hok help
Hok is a CLI implementation of Scoop in Rust

Usage: hok.exe <COMMAND>

Commands:
  bucket     Manage manifest buckets
  cache      List or remove download caches
  cat        Display manifest content of a package
  cleanup    Cleanup apps by removing old versions
  config     Configuration manipulations
  hold       Hold package(s) to disable updates
  home       Open the homepage of given package
  info       Display information about a package
  install    Install package(s)
  list       List installed package(s)
  search     Search available package(s)
  unhold     Unhold package(s) to enable updates
  uninstall  Uninstall package(s)
  update     Fetch and update all buckets
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
[license-badge]: https://img.shields.io/github/license/chawyehsu/hok
[downloads-badge]: https://img.shields.io/github/downloads/chawyehsu/hok/total
