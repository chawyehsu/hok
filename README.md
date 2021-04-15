# scoop-rs
The Scoop Windows command line installer rewritten in Rust

🚧 **Under construction, things may change without notice. Do NOT use it for production. Take care of your data!**

## Roadmap

**structure**: two parts, 1) Scoop API rust library and 2) Scoop CLI binary

**core features** implementation priority

- [x] bucket management (bucket)
- [x] cache management (cache)
- [ ] config management (config/alias)
- [ ] manifest manipulation (home/info/create)
- [ ] search functionality (search)
- [ ] app management (list/install/uninstall/update/cleanup/reset/hold/unhold)
- [ ] status subcommand (status)
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

## License

**scoop-rs** © Chawye Hsu, Released under the [Unlicense](UNLICENSE) License.

> [Website](https://chawyehsu.com) · GitHub [@chawyehsu](https://github.com/chawyehsu) · Twitter [@chawyehsu](https://twitter.com/chawyehsu)
