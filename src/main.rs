mod app;
mod game;
mod stats;
mod words;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
