use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys as dom;

pub fn tag(name: impl AsRef<str>) -> ElementBuilder {
    ElementBuilder::new(name)
}

pub struct ElementBuilder(Element);

impl ElementBuilder {
    pub fn new(tag: impl AsRef<str>) -> Self {
        ElementBuilder(Element {
            dom_element: DOCUMENT.with(|doc| doc.create_element(tag.as_ref()).unwrap()),
            states: Vec::new(),
        })
    }

    pub fn attribute(self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.0
            .dom_element
            .set_attribute(name.as_ref(), value.as_ref())
            .unwrap();
        self
    }

    pub fn child(mut self, child: impl Into<Element>) -> Self {
        let child = child.into();
        self.0.append_child(&child.dom_element);
        self.0.states.extend(child.states);
        self
    }

    pub fn text(self, child: impl AsRef<str>) -> Self {
        DOCUMENT.with(|doc| self.0.append_child(&doc.create_text_node(child.as_ref())));
        self
    }

    pub fn build(self) -> Element {
        self.0
    }
}

impl From<ElementBuilder> for Element {
    fn from(builder: ElementBuilder) -> Self {
        builder.build()
    }
}

pub struct Element {
    dom_element: dom::Element,
    states: Vec<Rc<dyn ChildState>>,
}

impl Element {
    pub fn append_to_body(&self) {
        DOCUMENT.with(|doc| {
            doc.body()
                .expect("Document must contain a `body`")
                .append_child(&self.dom_element)
                .unwrap()
        });
    }

    fn append_child(&self, node: &dom::Node) {
        self.dom_element.append_child(node).unwrap();
    }
}

pub fn state<T, E>(init: T, generate: impl 'static + Fn(T, StateSetter<T>) -> E) -> Element
where
    E: Into<Element>,
    T: 'static,
{
    let mut setter = StateSetter::default();
    // TODO: Can we pass setter by mut ref?
    let element = generate(init, setter.clone()).into();
    let dom_element = element.dom_element;
    let root_state = Rc::new(State {
        dom_element: RefCell::new(dom_element.clone()),
        generate: move |value, setter| generate(value, setter).into(),
        child_states: RefCell::new(element.states),
    });

    setter.updater.replace(root_state.clone());

    Element {
        dom_element,
        states: vec![root_state],
    }
}

impl<F> ChildState for State<F> {}

trait StateUpdater<T> {
    // TODO: Cancel children
    fn update(&self, new_value: T, setter: StateSetter<T>);
}

impl<T, F> StateUpdater<T> for State<F>
where
    F: 'static + Fn(T, StateSetter<T>) -> Element,
{
    fn update(&self, new_value: T, setter: StateSetter<T>) {
        let element = (self.generate)(new_value, setter);
        self.dom_element
            .borrow()
            .replace_with_with_node_1(&element.dom_element)
            .unwrap();
        self.dom_element.replace(element.dom_element);
        self.child_states.replace(element.states);
    }
}

trait AnyStateUpdater {
    fn update(&self);
}

impl<T> AnyStateUpdater for StateSetter<T> {
    /// # Panics
    ///
    /// If there is no new state with which to update,
    /// or if no updater has been set.
    fn update(&self) {
        let new_value = self.new_state.take().unwrap();

        self.updater
            .as_ref()
            .unwrap()
            .update(new_value, self.clone());
    }
}

pub struct StateSetter<T> {
    new_state: Rc<Cell<Option<T>>>,
    updater: Option<Rc<dyn StateUpdater<T>>>,
}

impl<T> Clone for StateSetter<T> {
    fn clone(&self) -> Self {
        Self {
            new_state: self.new_state.clone(),
            updater: self.updater.clone(),
        }
    }
}

impl<T> Default for StateSetter<T> {
    fn default() -> Self {
        Self {
            new_state: Rc::new(Cell::new(None)),
            updater: None,
        }
    }
}

impl<T: 'static> StateSetter<T> {
    pub fn set(&self, new_value: T) {
        if self.new_state.replace(Some(new_value)).is_none() {
            UPDATE_QUEUE.with(|update_queue| {
                let mut update_queue = update_queue.borrow_mut();

                update_queue.push(Box::new(self.clone()));

                if update_queue.len() == 1 {
                    request_process_updates();
                }
            });
        }
    }
}

trait ChildState {
    // TODO: Cancel method
}

struct State<F> {
    dom_element: RefCell<dom::Element>,
    generate: F,
    child_states: RefCell<Vec<Rc<dyn ChildState>>>,
}

fn window() -> dom::Window {
    dom::window().expect("Window must be available")
}

fn request_process_updates() {
    window()
        .request_animation_frame(
            Closure::once(Box::new(move |_time_stamp: JsValue| {
                process_updates();
            }))
            .as_ref()
            .unchecked_ref(),
        )
        .unwrap();
}

fn process_updates() {
    UPDATE_QUEUE.with(|update_queue| {
        let mut update_queue = update_queue.borrow_mut();

        for update in update_queue.drain(..) {
            // TODO: Rename update() to apply?
            update.update();
        }
    })
}

thread_local!(
    static DOCUMENT: dom::Document = window().document().expect("Window must contain a document");
    static UPDATE_QUEUE: RefCell<Vec<Box<dyn AnyStateUpdater>>> = RefCell::new(Vec::new());
);
