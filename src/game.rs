use std::sync::mpsc::Receiver;

use events::InputEvent;
use ticks::Ticker;

pub trait Game<T: Ticker> {
    fn new(event_loop: Receiver<InputEvent>, ticker: T) -> Self;
    fn main(&mut self);
}

