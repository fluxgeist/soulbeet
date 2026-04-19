use dioxus::prelude::*;
use ui::Library;

#[component]
pub fn LibraryPage() -> Element {
    rsx! {
        Library {}
    }
}
