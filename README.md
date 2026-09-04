# wesldoc

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](#license)
[![Build Status](https://github.com/jannik4/wesldoc/workflows/CI/badge.svg)](https://github.com/jannik4/wesldoc/actions)
[![Pages Status](https://github.com/jannik4/wesldoc/workflows/pages/badge.svg)](https://github.com/jannik4/wesldoc/actions)
[![dependency status](https://deps.rs/repo/github/jannik4/wesldoc/status.svg?path=crates%2Fwesldoc)](https://deps.rs/repo/github/jannik4/wesldoc?path=crates%2Fwesldoc) <!-- TODO: deps.rs does not support glob members in workspaces, so point to the "top" crate (https://github.com/deps-rs/deps.rs/issues/15) -->

Generate documentation for [WESL](https://github.com/wgsl-tooling-wg/wesl-spec) projects.

**Warning**: This is a work in progress and not stable in any way yet.

## Features

- Generate HTML documentation for WESL projects.
- Search for items or attributes in the documentation.
- Go to source code from the documentation.
- Documentation comments (`///` and `//!`) with Markdown formatting and support for intra-doc links.
- Show translate-time features in the documentation.
- Choose between a dark and a light theme.

For a live example, check out the [GitHub Pages site](https://jannik4.github.io/wesldoc/) of this repository.

## How to use

### Use the CLI

First install the `wesldoc` CLI:

<!-- TODO: Remove crates.io notice when it's published -->

```bash
cargo install wesldoc --locked --path ./crates/wesldoc # from this repository
cargo install wesldoc --locked --git https://github.com/jannik4/wesldoc # from git
cargo install wesldoc --locked # from crates.io (not yet published)
```

Then use it like this:

```bash
wesldoc ./path/to/my_wesl_project
```

Check `wesldoc --help` for more options.

> **Note**: Currently only `cargo` is supported as package manager. Support for other package managers may be added in the future.

### Use as a library

The `wesldoc` CLI is just a wrapper around the `wesldoc_resolver`, `wesldoc_compiler` and `wesldoc_generator` crates. You can use them directly in your own projects.
Look at the `wesldoc` crate for an example on how to use them.

## How it works

- [`wesldoc_ast`](crates/wesldoc_ast/): Provides the AST for the WESL documentation. It is fully standalone and does not depend on any `wesl` related crates.
- [`wesldoc_generator`](crates/wesldoc_generator/): Takes a `wesldoc_ast::WeslDocs` and generates the documentation in HTML format.
- [`wesldoc_resolver`](crates/wesldoc_resolver/): Resolves items and packages in a WESL project. It is agnostic to the underlying package manager used, e.g. [`wesldoc_resolver_cargo`](crates/wesldoc_resolver_cargo/) or [`wesldoc_resolver_npm`](crates/wesldoc_resolver_npm/).
- [`wesldoc_compiler`](crates/wesldoc_compiler/): Compiles a WESL package into a `WeslDocs` object using a `wesldoc_resolver::Resolver`.
- [`wesldoc`](crates/wesldoc/): Wrapper around `wesldoc_resolver`, `wesldoc_compiler` and `wesldoc_generator`. It provides a CLI to generate the documentation for WESL projects.

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

## Acknowledgements

Thanks to [docs.rs](https://docs.rs) for showing how great code documentation can look. `wesldoc` is heavily inspired by its design.
