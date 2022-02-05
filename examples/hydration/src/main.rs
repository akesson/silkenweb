//! A minimal interactive example
use futures_signals::signal::Mutable;
use silkenweb::{
    dom::{
        hydration::{hydrate_tracked, HydrationStats},
        render::spawn_local,
    },
    elements::{html::*, macros::ElementBuilder},
    prelude::*,
};

fn main() {
    let count = Mutable::new(0);
    let count_text = count.signal_ref(|i| format!("{}", i));
    let inc = move |_, _| {
        count.replace_with(|i| *i + 1);
    };

    let app = div().child(button().on_click(inc).text("+")).child(
        p().attribute("data-silkenweb-test", true)
            .text_signal(count_text),
    );

    spawn_local(async {
        let mut stats = HydrationStats::default();
        hydrate_tracked("app", app, &mut stats).await;
        web_log::println!("{}", stats);
    });
}
