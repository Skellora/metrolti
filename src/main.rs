extern crate metrolti_lib;

use metrolti_lib::server as server;
use metrolti_lib::metro_game as game;
use metrolti_lib::ticks::TPSTicker;
use metrolti_lib::web as web;

use std::thread;

pub fn main() {
    thread::spawn(|| web::startup_web_frontend("localhost:3005".to_string(), "localhost:3004".to_string(), "./www/static/".to_string()));
    server::listen::<game::MetroGame<TPSTicker>>("localhost:3004".to_string(), String::new());
}

