# WESL Docs

[![Build Status](https://github.com/jannik4/wesl_docs/workflows/CI/badge.svg)](https://github.com/jannik4/wesl_docs/actions)
[![Pages Status](https://github.com/jannik4/wesl_docs/workflows/pages/badge.svg)](https://github.com/jannik4/wesl_docs/actions)
[![dependency status](https://deps.rs/repo/github/jannik4/wesl_docs/status.svg?path=crates%2Fmake)](https://deps.rs/repo/github/jannik4/wesl_docs?path=crates%2Fmake) <!-- TODO: deps.rs does not support glob members in workspaces, so point to the "top" crate (https://github.com/deps-rs/deps.rs/issues/15) -->
[![Lines of Code](https://tokei.rs/b1/github/jannik4/wesl_docs)](https://github.com/jannik4/wesl_docs).
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Generate documentation for WESL projects.

**Warning**: This is a work in progress and not stable in any way yet.

## Features

- Generate HTML documentation for WESL projects.
- Search for items or attributes in the documentation.
- Go to source code from the documentation.
- Documentation comments (`///` and `//!`) with Markdown formatting and [(currently limited)](https://github.com/jannik4/wesl_docs/issues/3) support for intra-doc links.
- Show translate-time features in the documentation.
- Choose between a dark and a light theme.

For a live example, check out the [GitHub Pages site](https://jannik4.github.io/wesl_docs/) of this repository.

## How it works

- [`wesl_docs`](crates/wesl_docs/): This crate provides the AST for the WESL documentation. It is fully standalone and does not depend on `wesl-rs`.
- [`wesl_docs_generator`](crates/wesl_docs_generator/): This crate takes a `WeslDocs` from `wesl_docs` and generates the documentation in HTML format.
- [`wesl_docs_compiler`](crates/wesl_docs_compiler/): This crate takes the compile results from `wesl-rs` and compiles them into a `WeslDocs` object. It is agnostic to how the packages where resolved and compiled, but requires the availability of source maps to work properly.
- [`make`](crates/make/): This crate reads some example WESL packages from the disk, then resolves imports and compiles them using `wesl-rs`. Then it uses the `wesl_docs_compiler` to generate the documentation and the `wesl_docs_generator` to generate the HTML files.

## How to use

If you want to generate documentation for you own WESL projects, you can use the `wesl_docs_compiler` and `wesl_docs_generator` crates, which are meant to be used as libraries. Look at the `make` crate for an example on how to use them.

The workflow is currently still quite ad hoc, as there is not yet a standardized method for packaging WESL projects. As soon as this is the case, the API for creating the documentation will become more stable/straightforward.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
