# Unofficial Input Crate for the [SpacetimeDB](https://spacetimedb.com/) Rust Macro Bindings

This crate can be used if you want to develop your own [rust macros](https://doc.rust-lang.org/book/ch20-05-macros.html) on top of SpacetimeDB. It contains the parsing logic of the [spacetimedb-bindings-macro](https://github.com/clockworklabs/SpacetimeDB/tree/master/crates/bindings-macro) crate and therefore allows you to create your own logic based on the same input that SpacetimeDB itself receives when compiling the project.

## Motivation

This crate was originally created as part of a refactoring by a community member ([clockworklabs/SpacetimeDB#2626](https://github.com/clockworklabs/SpacetimeDB/pull/2626)) to simplify the development of his project [SpacetimeDSL](https://github.com/tamaro-skaljic/SpacetimeDSL).

However, it was rejected because the macro code had not been sufficiently tested and it was therefore not possible to be sure that the PR would not lead to errors.

## License

This crate and it's source code is licensed under the same license as SpacetimeDB, see [LICENSE](LICENSE).

SpacetimeDB is licensed under the BSL 1.1 license. This is not an open source or free software license, however, it converts to the AGPL v3.0 license with a linking exception after a few years.

Note that the AGPL v3.0 does not typically include a linking exception. We have added a custom linking exception to the AGPL license for SpacetimeDB. Our motivation for choosing a free software license is to ensure that contributions made to SpacetimeDB are propagated back to the community. We are expressly not interested in forcing users of SpacetimeDB to open source their own code if they link with SpacetimeDB, so we needed to include a linking exception.
