use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use events::{ InputEvent };
use game::Game;
use player_id::*;
use player::Player;
use ticks::*;

#[derive(Debug, Deserialize)]
pub enum PlayerAction {
    StartGame,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum StateUpdate {
    LobbyCount(u8),
    GameState(MetroModel),
}

// This would probably be better off with state-handling trait and types
#[derive(Debug, Eq, PartialEq)]
enum MGameState {
    Lobby,
    Game,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum StationType {
    Circle,
    //Triangle,
    Square,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct Station {
    t: StationType,
    position: (i8, i8),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct MetroModel {
    stations: Vec<Station>,
}

impl MetroModel {
    pub fn new() -> Self {
        Self {
            stations: Vec::new(),
        }
    }
}

pub struct MetroGame<T: Ticker> {
    state: MGameState,
    r: Receiver<InputEvent>,
    player_out: HashMap<PlayerId, Player>,
    ticker: T,
    model: MetroModel,
}

impl<T: Ticker> Game<T> for MetroGame<T> {
    fn new(event_loop: Receiver<InputEvent>, ticker: T) -> Self {
        MetroGame {
            state: MGameState::Lobby,
            r: event_loop,
            player_out: HashMap::new(),
            ticker: ticker,
            model: MetroModel::new(),
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
            MGameState::Game => self.handle_game_event(ev),
        }
    }

    fn handle_game_event(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::Connection(p_id, p) => {
                self.player_out.insert(p_id, p);
            }
            InputEvent::Disconnection(p_id) => {
                self.player_out.remove(&p_id);
            }
            InputEvent::PlayerAction(_p_id, _action) => { }
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
            InputEvent::PlayerAction(_p_id, action) => {
                match action {
                    PlayerAction::StartGame => { 
                        self.state = MGameState::Game;
                        self.model = MetroModel::new();
                        self.model.stations.push(Station { t: StationType::Circle, position: (10, -30) });
                        self.model.stations.push(Station { t: StationType::Square, position: (-50, 25) });
                    }
                }
            }
        }
    }
    pub fn update(&mut self) {}
    pub fn output(&mut self) {
        match self.state {
            MGameState::Lobby => self.lobby_output(),
            MGameState::Game => self.game_output(),
        }
    }
    pub fn game_output(&mut self) {
        for p in self.player_out.values() {
            p.send_message(StateUpdate::GameState(self.model.clone()));
        }
    }
    pub fn lobby_output(&mut self) {
        let connected = self.player_out.len();
        for p in self.player_out.values() {
            p.send_message(StateUpdate::LobbyCount(connected as u8));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{ channel, Sender, Receiver };
    use std::thread;
    use std::time::Duration;
    use ticks::TestTicker;
    use player::Player;
    use player_id::PlayerId;
    use events::*;
    use super::*;

    fn connect_player(sender: &Sender<InputEvent>, id: u16) -> Receiver<StateUpdate> {
        let (ps, pr) = channel();
        let p_id = PlayerId::new(id);
        let p = Player::new(ps);
        sender.send(InputEvent::Connection(p_id, p)).expect("test player connect");
        pr
    }

    fn disconnect_player(sender: &Sender<InputEvent>, id: u16) {
        sender.send(InputEvent::Disconnection(PlayerId::new(id))).expect("test pkayer disconnect");
    }

    fn start_test_game() -> (Sender<InputEvent>, (Sender<()>, Receiver<()>)) {
        let (tsw, trw) = channel();
        let (tss, trs) = channel();
        let t = TestTicker { r: trw, s: tss };
        let (gs, gr) = channel();
        let mut test_game = MetroGame::new(gr, t);
        thread::spawn(move || test_game.main());
        thread::sleep(Duration::from_millis(200));
        (gs, (tsw, trs))
    }

    fn send_player_action(sender: &Sender<InputEvent>, id: u16, action: PlayerAction) {
        sender.send(InputEvent::PlayerAction(PlayerId::new(id), action))
            .expect("Test sending player action");
    }

    fn tick(&(ref tsw, ref trs): &(Sender<()>, Receiver<()>)) {
        tsw.send(()).unwrap();
        trs.recv().unwrap();
    }

    #[test]
    fn connecting_players() {
        let (gs, ticks) = start_test_game();
        let pr1 = connect_player(&gs, 1);
        tick(&ticks);
        assert_eq!(Ok(StateUpdate::LobbyCount(1)), pr1.recv());
        let pr2 = connect_player(&gs, 2);
        tick(&ticks);
        assert_eq!(Ok(StateUpdate::LobbyCount(2)), pr1.recv());
        assert_eq!(Ok(StateUpdate::LobbyCount(2)), pr2.recv());
        disconnect_player(&gs, 1);
        tick(&ticks);
        assert!(pr1.try_recv().is_err());
        assert_eq!(Ok(StateUpdate::LobbyCount(1)), pr2.recv());
    }

    fn assert_is_game_start(update: &StateUpdate) {
        match *update {
            StateUpdate::GameState(ref state) => {
                assert_eq!(2, state.stations.len());
            }
            _ => {
                panic!("{:?} is not a GameState", update);
            }
        }
    }

    #[test]
    fn game_progression() {
        let (gs, ticks) = start_test_game();
        let pr1 = connect_player(&gs, 1);
        let pr2 = connect_player(&gs, 2);
        tick(&ticks);
        send_player_action(&gs, 1, PlayerAction::StartGame);
        tick(&ticks);
        assert_is_game_start(&pr1.recv().unwrap());
        assert_is_game_start(&pr2.recv().unwrap());
    }
}
