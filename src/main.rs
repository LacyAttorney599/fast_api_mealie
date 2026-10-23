//! « Ma Cuisine » : interface web au-dessus de Mealie, écrite en Rust et
//! compilée en WebAssembly. Pas de backend propre : le navigateur parle à
//! Mealie et Ollama à travers le reverse proxy qui sert l'application.

mod api;
mod app;
mod components;
mod pages;
mod py;
mod pyrandom;
mod util;

use wasm_bindgen::JsCast;

fn main() {
    console_error_panic_hook::set_once();
    let root = leptos::prelude::document()
        .get_element_by_id("root")
        .expect("index.html contient un élément #root")
        .unchecked_into::<web_sys::HtmlElement>();
    leptos::mount::mount_to(root, app::App).forget();
}
