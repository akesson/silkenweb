//! A library for building reactive single page web apps
//!
//! # Quick Start
//!
//! The best way to get started is to look at the examples. You'll need [trunk]
//! to run them. For example, to run [hello-world]:
//!
//! ```bash
//! cd examples/hello-world
//! trunk serve --open
//! ```
//!
//! - [hello-world] is a minimal example
//! - [counter] is a minimal interactive example
//! - [router] is a simple routing example
//! - [animation] is a simple animation example
//! - [todomvc] is an example of a simple app
//!
//! For a more complete introduction, see
//! [Learning Silkenweb With Entirely Too Many Counters](https://silkenweb.netlify.app/)
//!
//! [trunk]: https://trunkrs.dev/
//! [hello-world]: https://github.com/silkenweb/silkenweb/tree/main/examples/hello-world
//! [counter]: https://github.com/silkenweb/silkenweb/tree/main/examples/counter
//! [router]: https://github.com/silkenweb/silkenweb/tree/main/examples/router
//! [animation]: https://github.com/silkenweb/silkenweb/tree/main/examples/animation
//! [todomvc]: https://github.com/silkenweb/silkenweb/tree/main/examples/todomvc
pub use silkenweb_dom::{
    clone, mount,
    render::{after_render, render_updates},
    router, tag, tag_in_namespace, unmount, Attribute, AttributeValue, Builder, Element,
    ElementBuilder, Storage,
};
pub use silkenweb_html::{
    children_allowed, elements, html_element, CustomEvent, Effects, ElementEvents, HtmlElement,
    HtmlElementEvents, ParentBuilder,
};
pub use silkenweb_signals_ext::SignalProduct;

pub mod animation;
