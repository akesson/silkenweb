//! A library for building reactive web apps
//!
//! # Overview
//!
//! - Pure rust API
//! - Fine grained reactivity using [`futures_signals`]
//! - [Routing]
//! - [Tauri] support
//! - [Server Side Rendering] with hydration
//!
//! # Quick Start
//!
//! First, install the `wasm32` target:
//!
//! ```bash
//! rustup target add wasm32-unknown-unknown
//! ```
//!
//! Then install [trunk]:
//!
//! ```bash
//! cargo install trunk --locked
//! ```
//!
//! To run the example [counter]:
//!
//! ```bash
//! cd examples/counter
//! trunk serve --open
//! ```
//!
//! # Feature Flags
//!
//! ## `weak-refs`
//!
//! Use Javascript weak references to manage event callbacks. This improves
//! performance but must be enabled in `wasm-bindgen`. See the [trunk]
//! documentation for details on how to do this using `data-weak-refs`.
//!
//! See [caniuse](https://caniuse.com/mdn-javascript_builtins_weakref) for
//! current browser support.
//!
//! ## `declarative-shadow-dom`
//!
//! Print [Declarative Shadow DOM] when server side rendering. Hydration will
//! correctly deal with shadow DOM regardless of this flag. See
//! [caniuse](https://caniuse.com/mdn-html_elements_template_shadowroot)
//! for browser support. Polyfills are available.
//!
//! # Learning
//!
//! There's extensive documentation on each module in this crate, along with
//! many other examples in the [examples] folder.
//!
//! Reactivity is provided by [`futures_signals`]. It would be helpful to
//! familiarize yourself using [`futures_signals::tutorial`].
//!
//! [trunk]: https://trunkrs.dev/
//! [counter]: https://github.com/silkenweb/silkenweb/tree/main/examples/counter
//! [routing]: https://github.com/silkenweb/silkenweb/tree/main/examples/router
//! [tauri]: https://github.com/silkenweb/tauri-example
//! [Server Side Rendering]: https://github.com/silkenweb/ssr-example
//! [examples]: https://github.com/silkenweb/silkenweb/tree/main/examples
//! [Declarative Shadow DOM]: https://web.dev/declarative-shadow-dom/
use std::{cell::RefCell, collections::HashMap};

use dom::Wet;
use node::element::{Const, GenericElement};
#[doc(inline)]
pub use silkenweb_base::clone;
use silkenweb_base::document as base_document;
pub use silkenweb_macros::css;
/// Derive [`ChildElement`] and [`ChildNode`].
///
/// This only works for structs. It will defer to one field for the
/// implementation of the traits. If multiple fields are present, a target field
/// must be specified with `#[child_element(target)]`.
///
/// # Example
///
/// Derive traits for a newtype struct:
///
/// ```
/// # use silkenweb::{ChildElement, dom::InstantiableDom, node::Component};
/// #[derive(ChildElement)]
/// struct MyComponent<D: InstantiableDom>(Component<D>);
/// ```
///
/// Derive traits when the struct has more than 1 field:
///
/// ```
/// # use silkenweb::{ChildElement, dom::InstantiableDom, node::Component};
/// #[derive(ChildElement)]
/// struct MyComponent<D: InstantiableDom, Data> {
///     #[child_element(target)]
///     component: Component<D>,
///     data: Data,
/// }
/// ```
///
/// [`ChildElement`]: crate::node::element::ChildElement
/// [`ChildNode`]: crate::node::ChildNode
pub use silkenweb_macros::ChildElement;
/// Derive [`Element`].
///
/// This only works for structs. It will defer to one field for the
/// implementation. If multiple fields are present, a target field must be
/// specified with `#[element(target)]`.
///
/// # Example
///
/// Derive traits for a newtype struct:
///
/// ```
/// # use silkenweb::{dom::Dom, elements::html::Div, Element};
/// #
/// #[derive(Element)]
/// struct MyElement<D: Dom>(Div<D>);
/// ```
///
/// When the struct has more than 1 field:
///
/// ```
/// # use silkenweb::{dom::Dom, elements::html::Div, Element};
/// #
/// #[derive(Element)]
/// struct MyElement<D: Dom, Data> {
///     #[element(target)]
///     element: Div<D>,
///     data: Data,
/// }
/// ```
///
/// [`Element`]: crate::node::element::Element
pub use silkenweb_macros::Element;
#[doc(inline)]
pub use silkenweb_macros::{AriaElement, ElementEvents, HtmlElement, HtmlElementEvents, Value};
use wasm_bindgen::UnwrapThrowExt;

#[doc(hidden)]
#[macro_use]
pub mod macros;

pub mod animation;
pub mod attribute;
pub mod document;
pub mod dom;
pub mod elements;
pub mod hydration;
pub mod node;
pub mod router;
pub mod storage;
pub mod task;

/// Commonly used imports, all in one place.
pub mod prelude {
    pub use futures_signals::{
        signal::{Mutable, Signal, SignalExt},
        signal_vec::{MutableVec, SignalVec, SignalVecExt},
    };

    pub use crate::{
        clone,
        elements::{html, svg, AriaElement, ElementEvents, HtmlElement, HtmlElementEvents},
        mount,
        node::{
            element::{Element, ParentElement, ShadowRootParent},
            Node,
        },
        value::Sig,
    };
}

pub use silkenweb_signals_ext::value;

/// Mount an element on the document.
///
/// `id` is the html element id of the mount point. The element will replace the
/// mount point. The returned `MountHandle` should usually just be discarded,
/// but it can be used to restore the mount point if required. This can be
/// useful for testing.
pub fn mount(id: &str, element: impl Into<GenericElement<Wet, Const>>) -> MountHandle {
    let mut element = element.into();

    let mount_point = mount_point(id);
    element.mount(&mount_point);
    MountHandle::new(mount_point, element)
}

/// Remove all mounted elements.
///
/// Mount points will not be restored. This is useful to ensure a clean
/// environment for testing.
pub fn remove_all_mounted() {
    ELEMENTS.with(|elements| {
        for element in elements.take().into_values() {
            element.dom_element().remove()
        }
    });
}

/// Manage a mount point
pub struct MountHandle {
    id: u128,
    mount_point: web_sys::Element,
    on_drop: Option<DropAction>,
}

impl MountHandle {
    fn new(mount_point: web_sys::Element, element: GenericElement<Wet, Const>) -> Self {
        Self {
            id: insert_element(element),
            mount_point,
            on_drop: None,
        }
    }

    /// Stop the mounted element being reactive. This will free up any resources
    /// that are providing reactivity for the mounted element.
    pub fn stop(mut self) {
        self.stop_on_drop();
    }

    /// [`stop`][`Self::stop`] when `self` is dropped.
    pub fn stop_on_drop(&mut self) {
        self.on_drop = Some(DropAction::Stop);
    }

    /// Remove the mounted element and restore the mount point.
    ///
    /// Equivalent to calling [`stop`][`Self::stop`] and replacing the mounted
    /// element with the original mount point.
    pub fn unmount(mut self) {
        self.unmount_on_drop();
    }

    /// [`unmount`][`Self::unmount`] when `self` is dropped.
    pub fn unmount_on_drop(&mut self) {
        self.on_drop = Some(DropAction::Unmount);
    }
}

impl Drop for MountHandle {
    fn drop(&mut self) {
        match self.on_drop {
            Some(DropAction::Stop) => {
                remove_element(self.id);
            }
            Some(DropAction::Unmount) => {
                if let Some(element) = remove_element(self.id) {
                    element
                        .dom_element()
                        .replace_with_with_node_1(&self.mount_point)
                        .unwrap_throw();
                }
            }
            None => (),
        }
    }
}

enum DropAction {
    Stop,
    Unmount,
}

fn mount_point(id: &str) -> web_sys::Element {
    base_document::get_element_by_id(id)
        .unwrap_or_else(|| panic!("DOM node id = '{id}' must exist"))
}

fn insert_element(element: GenericElement<Wet, Const>) -> u128 {
    let id = next_node_handle_id();
    ELEMENTS.with(|elements| elements.borrow_mut().insert(id, element));
    id
}

fn remove_element(id: u128) -> Option<GenericElement<Wet, Const>> {
    ELEMENTS.with(|elements| elements.borrow_mut().remove(&id))
}

fn next_node_handle_id() -> u128 {
    ELEMENT_HANDLE_ID.with(|id| id.replace_with(|id| *id + 1))
}

thread_local!(
    static ELEMENT_HANDLE_ID: RefCell<u128> = RefCell::new(0);
    static ELEMENTS: RefCell<HashMap<u128, GenericElement<Wet, Const>>> =
        RefCell::new(HashMap::new());
);
