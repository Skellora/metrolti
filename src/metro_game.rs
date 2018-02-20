use std::sync::mpsc::Receiver;

use events::InputEvent;
use game::Game;

pub struct MetroGame {
}

impl Game for MetroGame {
    fn new(event_loop: Receiver<InputEvent>) -> Self {
        MetroGame {
        }
    }
    fn main(&mut self) {}
}

