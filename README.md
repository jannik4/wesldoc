# wesldoc

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](#license)
[![Build Status](https://github.com/jannik4/wesldoc/workflows/CI/badge.svg)](https://github.com/jannik4/wesldoc/actions)
[![Pages Status](https://github.com/jannik4/wesldoc/workflows/pages/badge.svg)](https://github.com/jannik4/wesldoc/actions)
[![dependency status](https://deps.rs/repo/github/jannik4/wesldoc/status.svg?path=crates%2Fwesldoc)](https://deps.rs/repo/github/jannik4/wesldoc?path=crates%2Fwesldoc) <!-- TODO: deps.rs does not support glob members in workspaces, so point to the "top" crate (https://github.com/deps-rs/deps.rs/issues/15) -->
[![Lines of Code](https://tokei.rs/b1/github/jannik4/wesldoc)](https://github.com/jannik4/wesldoc).

Generate documentation for [WESL](https://github.com/wgsl-tooling-wg/wesl-spec) projects.

**Warning**: This is a work in progress and not stable in any way yet.

## Features

- Generate HTML documentation for WESL projects.
- Search for items or attributes in the documentation.
- Go to source code from the documentation.
- Documentation comments (`///` and `//!`) with Markdown formatting and [(currently limited)](https://github.com/jannik4/wesldoc/issues/3) support for intra-doc links.
- Show translate-time features in the documentation.
- Choose between a dark and a light theme.

For a live example, check out the [GitHub Pages site](https://jannik4.github.io/wesldoc/) of this repository.

## How to use

### Use the CLI

First install the `wesldoc` CLI:

```bash
cargo install wesldoc --locked --path ./crates/wesldoc
```

Then use it like this:

```bash
wesldoc ./path/to/my_wesl_project -d ./path/to/dependency -d ./path/to/another_dependency
```

Check `wesldoc --help` for more options.

> **Note**: Dependencies are required to build the documentation, but are not built recursively, since there is not yet a standardized way to package WESL projects. This means you must run the `wesldoc` command for each dependency manually. This limitation will be lifted in the future, when a standardized way to package WESL projects is established.

### Use as a library

The `wesldoc` CLI is just a wrapper around the `wesldoc_compiler` and `wesldoc_generator` crates. You can use them directly in your own projects.
Look at the `wesldoc` crate for an example on how to use them.

## How it works

- [`wesldoc_ast`](crates/wesldoc_ast/): This crate provides the AST for the WESL documentation. It is fully standalone and does not depend on `wesl-rs`.
- [`wesldoc_generator`](crates/wesldoc_generator/): This crate takes a `WeslDocs` from `wesldoc_ast` and generates the documentation in HTML format.
- [`wesldoc_compiler`](crates/wesldoc_compiler/): This crate takes the compile results from `wesl-rs` and compiles them into a `WeslDocs` object. It is agnostic to how the packages where resolved and compiled, but requires the availability of source maps to work properly.
- [`wesldoc`](crates/wesldoc/): This crate is a wrapper around `wesldoc_compiler` and `wesldoc_generator`. It provides a CLI to generate the documentation from WESL packages. It uses `wesl-rs` to resolve and compile the packages, and then generates the documentation using `wesldoc_compiler` and `wesldoc_generator`.

> **Note**: Certain features of `wesldoc` may be migrated to `wesldoc_compiler` in the future once a standardized method for packaging WESL projects is established.

## Development

To build the example packages, run:

```bash
cargo run --example build_examples
```

## License

Licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
