# RLucene

<a href="README.md"><kbd>English</kbd></a>
<a href="README.zh-CN.md"><kbd>简体中文</kbd></a>

RLucene is a Rust port of [Apache Lucene](https://lucene.apache.org/), the
high-performance, full-featured search engine library.

The goal is to stay as close to Apache Lucene as possible—nearly 100% aligned
in architecture, behavior, semantics, and everyday usage. If you've used
Lucene before, RLucene should feel familiar. It keeps the same core concepts
and workflows, including documents, fields, analyzers, index writers, readers,
searchers, and queries.

A few small changes are unavoidable because Rust and Java handle types,
ownership, and errors differently.

RLucene currently tracks Apache Lucene 10.1. Once this port is feature-complete
and stable, the plan is to keep RLucene aligned with the Lucene release just
before the latest one.

## Project Status

RLucene is still under active development. Most commonly used features are
already implemented, but there is still plenty left to port, polish, and test.
Expect a few rough edges for now—we're working through them one by one.

For a general introduction to Lucene concepts and usage, see the
[Apache Lucene documentation](https://lucene.apache.org/core/documentation.html).

## Development

The Rust toolchain used by the project is pinned in
[`rust-toolchain.toml`](rust-toolchain.toml). If you use `rustup`, the right
toolchain and components will be selected automatically.

| Command | What it does |
| --- | --- |
| `cargo tidy` | Checks license headers, applies Cargo fixes, formats the code, and runs Clippy for all targets and features. You need to resolve all warnings and errors. |
| `cargo test-light` | Runs the same library test set as `cargo test`, but finishes faster by enabling light mode, reusing shared read-only fixtures, and reducing test debug information. Some expensive tests use a lighter workload in this mode. |
| `cargo commit` | Runs `cargo tidy` and fails on any unresolved warning or error, requires a clean working tree, and then runs `cargo test-light`. Use it after committing your changes and before submitting a PR; it does not create a Git commit. |
| `cargo nextest-run` | Runs tests with the third-party nextest test runner. |

### cargo-nextest

The `cargo nextest-run` command requires
[cargo-nextest](https://nexte.st/) to be installed first. This is a one-time
setup:

```shell
cargo install cargo-nextest --locked
```

You can then run:

```shell
cargo nextest-run
```

The other project-specific commands do not require cargo-nextest.

## License

RLucene is licensed under the [Apache License 2.0](LICENSE).
