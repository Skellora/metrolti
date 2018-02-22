use std::sync::mpsc::Receiver;
use std::time::{ Duration, Instant };
use std::thread;

pub trait Ticker {
    fn start(&mut self);
    fn wait_until_next_tick(&mut self);
}

pub struct TPSTicker {
    tps: Duration,
    last: Instant,
}

impl TPSTicker {
    pub fn new(tps: Duration) -> Self {
        TPSTicker {
            tps: tps,
            last: Instant::now(),
        }
    }
}

impl Ticker for TPSTicker {
    fn start(&mut self) {
        self.last = Instant::now();
    }

    fn wait_until_next_tick(&mut self) {
        let next_tick_wait = self.tps.checked_sub(self.last.elapsed());
        if let Some(wait) = next_tick_wait {
            thread::sleep(wait);
        }
        self.last = Instant::now();
    }
}

pub struct TestTicker {
    pub r: Receiver<()>,
}

impl Ticker for TestTicker {
    fn start(&mut self){}
    fn wait_until_next_tick(&mut self) {
        self.r.recv().unwrap();
    }
}
