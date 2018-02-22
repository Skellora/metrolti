use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use events::{ InputEvent, StateUpdate, PlayerAction };
use game::Game;
use player_id::*;
use player::Player;
use ticks::*;

// This would probably be better off with state-handling trait and types
#[derive(Debug, Eq, PartialEq)]
enum MGameState {
    Lobby,
    Game,
}

pub struct MetroGame<T: Ticker> {
    state: MGameState,
    r: Receiver<InputEvent>,
    player_out: HashMap<PlayerId, Player>,
    ticker: T,
}

impl<T: Ticker> Game<T> for MetroGame<T> {
    fn new(event_loop: Receiver<InputEvent>, ticker: T) -> Self {
        MetroGame {
            state: MGameState::Lobby,
            r: event_loop,
            player_out: HashMap::new(),
            ticker: ticker,
        }
    }
    fn main(&mut self) {
        self.ticker.start();
        loop {
            self.input();
            self.update();
            self.output();
            self.ticker.wait_until_next_tick();
        }
    }
}

impl<T: Ticker> MetroGame<T> {
    pub fn input(&mut self) {
        let mut events : Vec<_> = self.r.try_iter().collect();
        for in_event in events.drain(..) {
            self.handle_event(in_event);
        }
    }
    fn handle_event(&mut self, ev: InputEvent) {
        match self.state {
            MGameState::Lobby => self.handle_lobby_event(ev),
            MGameState::Game => {},
        }
    }

    fn handle_lobby_event(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::Connection(p_id, p) => {
                self.player_out.insert(p_id, p);
            }
            InputEvent::Disconnection(p_id) => {
                self.player_out.remove(&p_id);
            }
            InputEvent::PlayerAction(p_id, action) => {
                match action {
                    PlayerAction::StartGame => { self.state = MGameState::Game }
                    _ => {}
                }
            }
        }
    }
    pub fn update(&mut self) {}
    pub fn output(&mut self) {
        let connected = self.player_out.len();
        for p in self.player_out.values() {
            p.send_message(StateUpdate::LobbyCount(connected as u8));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;
    use std::thread;
    use ticks::TestTicker;
    use player::Player;
    use player_id::PlayerId;
    use events::*;
    use super::*;

    fn connect_player(sender: Sender<InputEvent>, id: u8) -> Receiver<StateUpdate> {
        let (ps, pr) = channel();
        let p_id = PlayerId::new(id);
        let p = Player::new(ps);
        sender.send(InputEvent::Connection(p_id, p));
        pr
    }

    fn start_test_game() -> (Sender<InputEvent>, Sender<()>) {
        let (ts, tr) = channel();
        let t = TestTicker { r: tr };
        let (gs, gr) = channel();
        let mut test_game = MetroGame::new(gr, t);
        thread::spawn(move || test_game.main());
        (gs, ts)
    }

    #[test]
    fn tada() {
        let (gs, ts) = start_test_game();
        let pr1 = connect_player(gs, 1);
        ts.send(());
        assert_eq!(StateUpdate::LobbyCount(1), pr1.recv().unwrap());
        let pr2 = connect_player(gs, 2);
        ts.send(());
        assert_eq!(StateUpdate::LobbyCount(2), pr1.recv().unwrap());
        assert_eq!(StateUpdate::LobbyCount(2), pr2.recv().unwrap());
    }
}
