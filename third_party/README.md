# Third-party code kept in the repository

Both folders are taken over as they are from [ps5upload](https://github.com/phantomptr/ps5upload)
(`third_party/unrar` and `third_party/sevenz-rust2` at 583cb971), with thanks to phantomptr: each
is an upstream crate with one fix ps5upload made, described in its own `README.md` (the paths
named there are ps5upload's). Here the top `Cargo.toml` redirects to them with
`[patch.crates-io]`.

- `unrar` 0.5.8 (<https://github.com/muja/unrar.rs>, MIT OR Apache-2.0): only `crates/ugbm-unrar`
  links it, never the app (`LICENSES/UnRAR-exception.md`).
- `sevenz-rust2` 0.23.0 (<https://github.com/hasenbanck/sevenz-rust2>, Apache-2.0): the app
  reads 7z archives with it (`src/library/sevenz.rs`).

To update: `git -C ~/Repositories/ps5upload pull`, then copy the folders again and note the new
commit here.
