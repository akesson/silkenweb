//! A minimal interactive example
use silkenweb::{
    elements::{button, div, p},
    mount,
    signal::Signal,
};

fn main() {
    let count = Signal::new(0);
    let set_count = count.write();
    let inc = move |_, _| set_count.replace(|&i| i + 1);
    let count_text = count.read().map(|i| format!("{}", i));

    let app = div()
        .child(button().on_click(inc).text("+"))
        .child(p().text(count_text));

    mount("app", app);
}