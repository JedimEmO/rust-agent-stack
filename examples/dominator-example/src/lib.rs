#[macro_use]
extern crate dominator;

use dominator::Dom;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
pub fn main() {
    dominator::append_dom(&dominator::body(), render());
}

fn render() -> Dom {
    html!("div", {
        .text("hi")
    })
}