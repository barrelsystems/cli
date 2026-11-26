# Contributing

## Reporting a bug or requesting a feature

Open a [GitHub issue](https://github.com/barrelsystems/cli/issues) with
steps to reproduce (for a bug) or the use case you're trying to cover
(for a feature).

## Local development

You'll need a stable Rust toolchain with the `clippy` and `rustfmt`
components.

```
cargo build
cargo test
```

## Before opening a pull request

Run the same checks CI runs:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

All four must pass clean. `cargo fmt` (without `--check`) will fix
formatting issues for you.

## Pull requests

Open your PR against `master`. Keep it focused — a PR that does one thing
is easier to review than one that bundles unrelated changes. Describe what
changed and why in the PR description.

By contributing, you agree your contribution is licensed under this
repository's [Apache-2.0 license](LICENSE).

## Code of conduct

Participation in this project is governed by our
[Code of Conduct](CODE_OF_CONDUCT.md).
