# WESL Docs

Generate documentation for WESL projects.

**Warning**: This is a work in progress and not stable in any way yet.

## How it works

- [`wesl_docs`](crates/wesl_docs/): This crate provides the AST for the WESL documentation. It is fully standalone and does not depend on `wesl-rs`.
- [`wesl_docs_generator`](crates/wesl_docs_generator/): This crate takes a `WeslDocs` from `wesl_docs` and generates the documentation in HTML format.
- [`wesl_docs_compiler`](crates/wesl_docs_compiler/): This crate takes the compile results from `wesl-rs` and compiles them into a `WeslDocs` object. It is agnostic to how the packages where resolved and compiled.
- [`make`](crates/make/): This crate reads some example WESL packages from the disk, then resolves imports and compiles them using `wesl-rs`. Then it uses the `wesl_docs_compiler` to generate the documentation and the `wesl_docs_generator` to generate the HTML files.

## How to use

If you want to generate documentation for you own WESL projects, you can use the `wesl_docs_compiler` and `wesl_docs_generator` crates, which are meant to be used as libraries. Look at the `make` crate for an example on how to use them.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
