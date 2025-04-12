# wesldoc

[![Build Status](https://github.com/jannik4/wesldoc/workflows/CI/badge.svg)](https://github.com/jannik4/wesldoc/actions)
[![Pages Status](https://github.com/jannik4/wesldoc/workflows/pages/badge.svg)](https://github.com/jannik4/wesldoc/actions)
[![dependency status](https://deps.rs/repo/github/jannik4/wesldoc/status.svg?path=crates%2Fmake)](https://deps.rs/repo/github/jannik4/wesldoc?path=crates%2Fmake) <!-- TODO: deps.rs does not support glob members in workspaces, so point to the "top" crate (https://github.com/deps-rs/deps.rs/issues/15) -->
[![Lines of Code](https://tokei.rs/b1/github/jannik4/wesldoc)](https://github.com/jannik4/wesldoc).
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Generate documentation for WESL projects.

**Warning**: This is a work in progress and not stable in any way yet.

## Features

- Generate HTML documentation for WESL projects.
- Search for items or attributes in the documentation.
- Go to source code from the documentation.
- Documentation comments (`///` and `//!`) with Markdown formatting and [(currently limited)](https://github.com/jannik4/wesldoc/issues/3) support for intra-doc links.
- Show translate-time features in the documentation.
- Choose between a dark and a light theme.

For a live example, check out the [GitHub Pages site](https://jannik4.github.io/wesldoc/) of this repository.

## How it works

- [`wesldoc_ast`](crates/wesldoc_ast/): This crate provides the AST for the WESL documentation. It is fully standalone and does not depend on `wesl-rs`.
- [`wesldoc_generator`](crates/wesldoc_generator/): This crate takes a `WeslDocs` from `wesldoc_ast` and generates the documentation in HTML format.
- [`wesldoc_compiler`](crates/wesldoc_compiler/): This crate takes the compile results from `wesl-rs` and compiles them into a `WeslDocs` object. It is agnostic to how the packages where resolved and compiled, but requires the availability of source maps to work properly.
- [`make`](crates/make/): This crate reads some example WESL packages from the disk, then resolves imports and compiles them using `wesl-rs`. Then it uses the `wesldoc_compiler` to generate the documentation and the `wesldoc_generator` to generate the HTML files.

## How to use

If you want to generate documentation for you own WESL projects, you can use the `wesldoc_compiler` and `wesldoc_generator` crates, which are meant to be used as libraries. Look at the `make` crate for an example on how to use them.

The workflow is currently still quite ad hoc, as there is not yet a standardized method for packaging WESL projects. As soon as this is the case, the API for creating the documentation will become more stable/straightforward.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
