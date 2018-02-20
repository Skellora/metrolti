extern crate metrolti_lib;

use metrolti_lib::server as server;
use metrolti_lib::metro_game as game;

pub fn main() {
    server::listen::<game::MetroGame>("localhost:3004".to_string(), String::new());
}

