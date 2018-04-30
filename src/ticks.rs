use std::sync::mpsc::{ Receiver, Sender };
use std::time::{ Duration, Instant };
use std::thread;

pub trait Ticker {
    fn start(&mut self);
    fn wait_until_next_tick(&mut self) -> f64;
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

    fn wait_until_next_tick(&mut self) -> f64 {
        let next_tick_wait = self.tps.checked_sub(self.last.elapsed());
        if let Some(wait) = next_tick_wait {
            thread::sleep(wait);
        } else {
            println!("Server can't keep up");
        }
        let time_waited = self.last.elapsed();
        self.last = Instant::now();
        time_waited.as_secs() as f64 + (time_waited.subsec_nanos() as f64 / 1_000_000_000_f64)
    }
}

pub struct TestTicker {
    pub r: Receiver<f64>,
    pub s: Sender<()>,
}

impl Ticker for TestTicker {
    fn start(&mut self){}
    fn wait_until_next_tick(&mut self) -> f64 {
        self.s.send(()).expect("test ticker signal");
        let s = self.r.recv().expect("test ticker wait");
        s
    }
}
