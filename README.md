# lignin-html

[![Lib.rs](https://img.shields.io/badge/Lib.rs-*-84f)](https://lib.rs/crates/lignin-html)
[![Crates.io](https://img.shields.io/crates/v/lignin-html)](https://crates.io/crates/lignin-html)
[![Docs.rs](https://docs.rs/lignin-html/badge.svg)](https://docs.rs/lignin-html)

![Rust 1.54](https://img.shields.io/static/v1?logo=Rust&label=&message=1.54&color=grey)
[![CI](https://github.com/Tamschi/lignin-html/workflows/CI/badge.svg?branch=develop)](https://github.com/Tamschi/lignin-html/actions?query=workflow%3ACI+branch%3Adevelop)
![Crates.io - License](https://img.shields.io/crates/l/lignin-html/0.0.5)

[![GitHub](https://img.shields.io/static/v1?logo=GitHub&label=&message=%20&color=grey)](https://github.com/Tamschi/lignin-html)
[![open issues](https://img.shields.io/github/issues-raw/Tamschi/lignin-html)](https://github.com/Tamschi/lignin-html/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/Tamschi/lignin-html)](https://github.com/Tamschi/lignin-html/pulls)
[![good first issues](https://img.shields.io/github/issues-raw/Tamschi/lignin-html/good%20first%20issue?label=good+first+issues)](https://github.com/Tamschi/lignin-html/contribute)

[![crev reviews](https://web.crev.dev/rust-reviews/badge/crev_count/lignin-html.svg)](https://web.crev.dev/rust-reviews/crate/lignin-html/)
[![Zulip Chat](https://img.shields.io/endpoint?label=chat&url=https%3A%2F%2Fiteration-square-automation.schichler.dev%2F.netlify%2Ffunctions%2Fstream_subscribers_shield%3Fstream%3Dproject%252Flignin-html)](https://iteration-square.schichler.dev/#narrow/stream/project.2Flignin-html)

HTML renderer for [lignin] VDOM Nodes.

This crate is primarily for static and server-side rendering.  
For client-side use, see [lignin-dom].

[lignin]: https://github.com/Tamschi/lignin
[lignin-dom]: https://github.com/Tamschi/lignin-dom

## Installation

Please use [cargo-edit](https://crates.io/crates/cargo-edit) to always add the latest version of this library:

```cmd
cargo add lignin-html
```

## Example

```rust
use lignin::{Node, Element, ElementCreationOptions};
use lignin_html::render_document;

let mut document = String::new();

render_document(
  &Node::HtmlElement {
    element: &Element {
      // `lignin-html` is case-preserving but insensitive.
      // `lignin-dom` slightly prefers all-caps, as may other DOM renderers.¹
      name: "DIV",
      creation_options: ElementCreationOptions::new(), // `const fn` builder pattern.
      attributes: &[],
      content: Node::Multi(&[
        "Hello! ".into(),
        Node::Comment {
          // `lignin-html` escapes or substitutes where necessary and unobtrusive.
          comment: "--> Be mindful of HTML pitfalls. <!--",
          dom_binding: None,
        }
      ]),
      event_bindings: &[],
    },
    dom_binding: None, // Bindings are inactive with `lignin-html`.
  }
  .prefer_thread_safe(),
  &mut document,
  1000, // depth_limit
).unwrap(); // Invalid content that can't be escaped is rejected.

assert_eq!(
  document,
  "<!DOCTYPE html><DIV>Hello! <!--==> Be mindful of HTML pitfalls. <!==--></DIV>",
);
```

¹ See [Element.tagName (MDN)](https://developer.mozilla.org/en-US/docs/Web/API/Element/tagName). This avoids case-insensitive comparisons, but can cause elements to be recreated during hydration if the reported ***tagName*** doesn't quite match the generated `name`.

## License

Licensed under either of

- Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING](CONTRIBUTING.md) for more information.

## [Code of Conduct](CODE_OF_CONDUCT.md)

## [Changelog](CHANGELOG.md)

## Versioning

`lignin-html` strictly follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html) with the following exceptions:

- The minor version will not reset to 0 on major version changes (except for v1).  
Consider it the global feature level.
- The patch version will not reset to 0 on major or minor version changes (except for v0.1 and v1).  
Consider it the global patch level.

This includes the Rust version requirement specified above.  
Earlier Rust versions may be compatible, but this can change with minor or patch releases.

Which versions are affected by features and patches can be determined from the respective headings in [CHANGELOG.md](CHANGELOG.md).

Note that dependencies of this crate may have a more lenient MSRV policy!
Please use `cargo +nightly update -Z minimal-versions` in your automation if you don't generate Cargo.lock manually (or as necessary) and require support for a compiler older than current stable.
