use std::sync::mpsc::Receiver;
use rand::{Rng, thread_rng};

pub trait Random {
    fn gen(&self) -> f64;
}

pub struct Always1Random;

impl Random for Always1Random {
    fn gen(&self) -> f64 {
        1f64
    }
}

pub struct TestRandom {
    pub r: Receiver<f64>,
}

impl Random for TestRandom {
    fn gen(&self) -> f64 {
        self.r.recv().unwrap()
    }
}

pub struct RealRandom;

impl Random for RealRandom {
    fn gen(&self) -> f64 {
        thread_rng().gen()
    }
}

