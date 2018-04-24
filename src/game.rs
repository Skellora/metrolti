use std::sync::mpsc::Receiver;

use events::InputEvent;
use ticks::Ticker;
use randoms::Random;

pub trait Game<T: Ticker, R: Random> {
    fn new(event_loop: Receiver<InputEvent>, ticker: T, random: R) -> Self;
    fn main(&mut self);
}

