# AsParIt - Async Parallel Iterators

[![Rayon crate](https://img.shields.io/crates/v/asparit.svg)](https://crates.io/crates/asparit)
[![Rayon documentation](https://docs.rs/asparit/badge.svg)](https://docs.rs/asparit)

Asparit implements async parallel iterators. It is mostly based on the idea and the code of
[rayon][rayon]. If you need some more detailed information please refere to the documentation of
[rayon][rayon]. The benefit of asparit is, that the iterator can be driven by a so called executor.
For now three different executors are implemented:
- rayon - Use rayon as executor.
- tokio - Use tokio as executor. This enabled the iterator to support async/await syntax.
- sequential - Simple executor that drives the iterator sequential. This may be usefull for
testing purposes.

[rayon]: https://github.com/rayon-rs/rayon
[tokio]: https://github.com/tokio-rs/tokio

## Example

Simple example to use asparit with the tokio executor.

```rust
use asparit::*;

let s = (0..10)
    .into_par_iter()
    .sum()
    .exec_with(TokioExecutor::default())
    .await;

assert_eq!(s, 45);
```

If tokio is setup as the default executor, you can simply use `exec` instead of `exec_with`.

```rust
use asparit::*;

let s = (0..10)
    .into_par_iter()
    .sum()
    .exec()
    .await;

assert_eq!(s, 45);
```

## License

Asparit is distributed under the terms of both the MIT license and the
Apache License (Version 2.0). See [LICENSE-APACHE](LICENSE-APACHE) and
[LICENSE-MIT](LICENSE-MIT) for details. Opening a pull requests is
assumed to signal agreement with these licensing terms.
