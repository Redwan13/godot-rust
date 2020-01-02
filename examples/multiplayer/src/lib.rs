mod bomb;
mod gamestate;
mod lobby;
mod util;

use bomb::Bomb;
use gamestate::GameState;
use gdnative::*;
use lobby::Lobby;

fn init(handle: init::InitHandle) {
    handle.add_class::<Bomb>();
    handle.add_class::<GameState>();
    handle.add_class::<Lobby>();
}

godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
