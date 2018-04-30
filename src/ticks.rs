use std::sync::mpsc::{ Receiver, Sender };
use std::time::{ Duration, Instant };
use std::thread;

pub trait Ticker {
    fn start(&mut self);
    fn wait_until_next_tick(&mut self) -> Duration;
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

    fn wait_until_next_tick(&mut self) -> Duration {
        let next_tick_wait = self.tps.checked_sub(self.last.elapsed());
        if let Some(wait) = next_tick_wait {
            thread::sleep(wait);
        } else {
            println!("Server can't keep up");
        }
        let time_waited = self.last.elapsed();
        self.last = Instant::now();
        time_waited
    }
}

pub struct TestTicker {
    pub r: Receiver<u64>,
    pub s: Sender<()>,
}

impl Ticker for TestTicker {
    fn start(&mut self){}
    fn wait_until_next_tick(&mut self) -> Duration {
        self.s.send(()).expect("test ticker signal");
        let m = self.r.recv().expect("test ticker wait");
        Duration::from_millis(m)
    }
}
