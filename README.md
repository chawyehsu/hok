# scoop-rs

[![](https://img.shields.io/badge/Telegram-Group-0067B8.svg?style=flat-square&logo=telegram&color=0088cc&labelColor=282c34&longCache=true)](https://t.me/scoop_rs)

The Scoop Windows command line installer rewritten in Rust

🚧 **Under heavy development, things may change without notice. Do NOT use it for production. Take care of your data!**

## Roadmap

**structure**: two parts, 1) Scoop API rust library and 2) Scoop CLI binary

**core features** implementation priority

- [x] bucket management (bucket)
- [x] cache management (cache)
- [x] config management (config)
- [x] manifest manipulation (home/info)
- [x] search functionality (search)
- [ ] app management (list/install/uninstall/update/cleanup/reset/hold/unhold)
- [x] status subcommand (status)
- [ ] shimming feature
- [ ] other subcommands (checkup/prefix/which/virustotal/etc.)

## Development

Prerequisites: Git, Rust

```sh
# clone the repo
git clone https://github.com/chawyehsu/scoop-rs
cd scoop-rs
# build
cargo build
# run and test
.\target\debug\scoop.exe
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>
