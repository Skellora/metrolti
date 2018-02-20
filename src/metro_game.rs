use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::{ Instant, Duration };
use std::thread;

use events::{ InputEvent, StateUpdate };
use game::Game;
use player_id::*;
use player::Player;

// This would probably be better off with state-handling trait and types
enum MGameState {
    Lobby,
    Game,
}

pub struct MetroGame {
    state: MGameState,
    r: Receiver<InputEvent>,
    player_out: HashMap<PlayerId, Player>,
}

impl Game for MetroGame {
    fn new(event_loop: Receiver<InputEvent>) -> Self {
        MetroGame {
            state: MGameState::Lobby,
            r: event_loop,
            player_out: HashMap::new(),
        }
    }
    fn main(&mut self) {
        let tick_rate = Duration::from_millis(1000/30);
        let mut last_update = Instant::now();
        loop {
            self.input();
            self.update();
            self.output();
            let next_tick_wait = tick_rate.checked_sub(last_update.elapsed());
            if let Some(wait) = next_tick_wait {
                thread::sleep(wait);
            }
            last_update = Instant::now();
        }
    }
}

impl MetroGame {
    pub fn input(&mut self) {
        let in_event = self.r.recv();
        match in_event {
            Ok(e) => self.handle_event(e),
            Err(_) => self.quit(),
        }
        
    }
    fn handle_event(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::Connection(p_id, p) => {
                self.player_out.insert(p_id, p);
            }
            InputEvent::Disconnection(p_id) => {
                self.player_out.remove(&p_id);
            }
            _ => {}
        }
    }
    fn quit(&mut self) {
        panic!("Gameover");
    }
    pub fn update(&mut self) {}
    pub fn output(&mut self) {
        let connected = self.player_out.len();
        for p in self.player_out.values() {
            p.send_message(StateUpdate::LobbyCount(connected as u8));
        }
    }
}

