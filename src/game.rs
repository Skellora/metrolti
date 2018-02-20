use std::sync::mpsc::Receiver;

use events::InputEvent;

pub trait Game {
    fn new(event_loop: Receiver<InputEvent>) -> Self;
    fn main(&mut self);
}

