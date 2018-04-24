use std::sync::mpsc::Receiver;
use rand::{Rng, thread_rng};

pub trait Random {
    fn gen(&self) -> f32;
}

pub struct TestRandom {
    pub r: Receiver<f32>,
}

impl Random for TestRandom {
    fn gen(&self) -> f32 {
        self.r.recv().unwrap()
    }
}

pub struct RealRandom;

impl Random for RealRandom {
    fn gen(&self) -> f32 {
        thread_rng().gen()
    }
}

