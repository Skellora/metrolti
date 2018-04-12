use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use rand::{Rng, thread_rng};

use events::{ InputEvent };
use game::Game;
use player_id::*;
use player::Player;
use ticks::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum PlayerAction {
    StartGame,
    ConnectStations(StationId, StationId),
}

#[derive(Debug, PartialEq, Serialize)]
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
    Triangle,
    Square,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct StationId(pub usize);

pub type Point = (f32, f32);

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Station {
    t: StationType,
    position: Point,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Edge {
    origin: StationId,
    destination: StationId,
    via_point: Point,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct LineId(pub usize);

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Line {
    colour: (f64, f64, f64),
    edges: Vec<Edge>,
    owning_player: PlayerId,
}

enum GetEdgeResult<'a> {
    Edge(&'a Edge),
    LocNotFound,
    EndOfTheLine,
}

impl Line {
    fn get_edge_after_station(&self, id: &StationId) -> GetEdgeResult {
        for e in self.edges.iter() {
            if e.origin == *id {
                return GetEdgeResult::Edge(e);
            }
        }
        if let Some(last_edge) = self.edges.last() {
            if last_edge.destination == *id {
                return GetEdgeResult::EndOfTheLine;
            }
        }
        GetEdgeResult::LocNotFound
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TrainId(pub usize);

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Train {
    on_line: LineId,
    position: Point,
    heading: Point,
    forward: bool,
    between_stations: (StationId, StationId),
    speed: f32,
}

enum TrainNextTarget {
    Heading(Point),
    Reverse(Point),
    Edge(StationId, Point, StationId),
    None,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct MetroModel {
    stations: Vec<Station>,
    lines: Vec<Line>,
    trains: Vec<Train>,
    station_size: u8,
}

impl MetroModel {
    pub fn new() -> Self {
        Self {
            stations: Vec::new(),
            station_size: 20u8,
            lines: Vec::new(),
            trains: Vec::new(),
        }
    }
    pub fn get_station(&self, id: &StationId) -> Option<&Station> {
        let &StationId(index) = id;
        self.stations.get(index)
    }

    pub fn get_train(&self, id: &TrainId) -> Option<&Train> {
        let &TrainId(index) = id;
        self.trains.get(index)
    }
    pub fn get_train_mut(&mut self, id: &TrainId) -> Option<&mut Train> {
        let &TrainId(index) = id;
        self.trains.get_mut(index)
    }

    pub fn get_station_pos(&self, id: &StationId) -> Option<Point> {
        self.get_station(id).map(|s| s.position)
    }

    pub fn get_line(&self, id: &LineId) -> Option<&Line> {
        let &LineId(index) = id;
        self.lines.get(index)
    }

    pub fn add_train_to_line(&mut self, id: &LineId) {
        let &LineId(index) = id;
        let line = self.lines.get(index);
        let (station_pair, via) = if let Some(line) = line {
            ((line.edges[0].origin.clone(), line.edges[0].destination.clone()), line.edges[0].via_point)
        } else {
            return
        };
        let train = Train {
            on_line: id.clone(),
            position: self.get_station_pos(&station_pair.0).unwrap_or((0f32,0f32)),
            heading: via,
            forward: true,
            between_stations: station_pair,
            speed: 0.2,
        };
        self.trains.push(train);
    }

    pub fn add_edge_to_line(&mut self, id: &LineId, edge: Edge) {
        let &LineId(index) = id;
        let line = self.lines.get_mut(index);
        if let Some(line) = line {
            line.edges.push(edge);
        }
    }

    fn step_train(&mut self, id: &TrainId) {
        let train = match self.get_train_mut(id) {
            Some(t) => t,
            None => return,
        };
        let point_proximity = 0.2;
        let dx = train.heading.0 - train.position.0;
        let dy = train.heading.1 - train.position.1;
        let sqr_dist = dx * dx + dy * dy;
        let dist = sqr_dist.sqrt();
        if dist > point_proximity {
            println!("Dist: {}", dist);
            train.position.0 += dx.signum() * train.speed;
            train.position.1 += dy.signum() * train.speed;
        } else if dist > 0. {
            train.position = train.heading.clone();
            println!("Stopped at {:?}", train.position);
        } 
    }

    fn train_reached_destination(&self, id: &TrainId) -> bool {
        match self.get_train(id) {
            Some(t) => t.position.0 == t.heading.0 && t.position.1 == t.heading.1,
            None => false,
        }
    }

    fn get_train_next_destination(&self, id: &TrainId) -> TrainNextTarget {
        if let Some(t) = self.get_train(id) {
            let target_id = &t.between_stations.1;
            if let Some(target_pos) = self.get_station_pos(target_id) {
                if t.heading.0 != target_pos.0 || t.heading.1 != target_pos.1 {
                    return TrainNextTarget::Heading(target_pos);
                }
                if let Some(line) = self.get_line(&t.on_line) {
                    match line.get_edge_after_station(target_id) {
                        GetEdgeResult::Edge(e) =>
                            return TrainNextTarget::Edge(e.origin.clone(), e.via_point, e.destination.clone()),
                        GetEdgeResult::LocNotFound =>
                            return TrainNextTarget::None,
                        GetEdgeResult::EndOfTheLine =>
                            return TrainNextTarget::Reverse(line.edges.last().unwrap().via_point),
                    }
                }
            }
        }
        TrainNextTarget::None
    }

    fn update_train(&mut self, id : &TrainId) {
        self.step_train(id);
        if !self.train_reached_destination(id) {
            return;
        }
        let next_dest = self.get_train_next_destination(id);
        if let Some(t) = self.get_train_mut(id) {
            match next_dest {
                TrainNextTarget::Heading(p) => {
                    t.heading = p;
                }
                TrainNextTarget::Reverse(p) => {
                    t.heading = p;
                    t.forward = !t.forward;
                }
                TrainNextTarget::Edge(origin, p, dest) => {
                    t.heading = p;
                    t.between_stations = (origin, dest);
                }
                TrainNextTarget::None => {},
            }
        }
    }

    pub fn update(&mut self) {
        for i in 0..self.trains.len() {
            let id = TrainId(i);
            self.update_train(&id);
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
            InputEvent::PlayerAction(p_id, action) => { 
                match action {
                    PlayerAction::ConnectStations(src, tgt) => {
                        let via = self.get_via_point_between(&src, &tgt);
                        let mut line_id = LineId(0);
                        for (index, line) in self.model.lines.iter().enumerate() {
                            if line.owning_player != p_id { continue; }
                            line_id = LineId(index);
                        }
                        self.model.add_edge_to_line(&line_id, Edge { origin: src.clone(), destination: tgt.clone(), via_point: via.clone() });
                        self.model.add_train_to_line(&line_id);
                    }
                    PlayerAction::StartGame => {
                        // Game is already started
                    }
                }
            }
        }
    }
    fn get_via_point_between(&self, origin: &StationId, destination: &StationId) -> Point {
        let (origin_x, origin_y) = self.model.get_station(origin).map(|s| s.position).unwrap_or((0f32, 0f32));
        let (dest_x, dest_y) = self.model.get_station(destination).map(|s| s.position).unwrap_or_default();
        let dx = dest_x - origin_x;
        let dy = dest_y - origin_y;
        let diag;
        if dx.abs() < dy.abs() {
            diag = (dx, dx.abs() * dy.signum());
        } else {
            diag = (dy.abs() * dx.signum(), dy);
        }
        (origin_x + diag.0, origin_y + diag.1)
    }

    fn get_player_ids(&self) -> Vec<PlayerId> {
        self.player_out.keys().cloned().collect()
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
                        self.model.stations.push(Station { t: StationType::Circle, position: (10., -30.) });
                        self.model.stations.push(Station { t: StationType::Square, position: (-45., 70.) });
                        self.model.stations.push(Station { t: StationType::Triangle, position: (300., 30.) });
                        let mut rng = thread_rng();
                        for player in self.get_player_ids() {
                            self.model.lines.push(Line { edges: Vec::new(), colour: (rng.gen(), rng.gen(), rng.gen()), owning_player: player });
                        }
                    }
                    _ => {
                        // It's unlikelu that there will be any more events that
                        // have an effect in the lobby
                    }
                }
            }
        }
    }
    pub fn update(&mut self) {
        self.model.update();
    }
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
                assert_eq!(3, state.stations.len());
                for line in state.lines.iter() {
                    assert_eq!(0, line.edges.len());
                }
            }
            _ => {
                panic!("{:?} is not a GameState", update);
            }
        }
    }
    fn assert_has_edge(update: &StateUpdate, src: &StationId, tgt: &StationId) {
        match *update {
            StateUpdate::GameState(ref state) => {
                for line in state.lines.iter() {
                    for edge in line.edges.iter() {
                        if edge.origin == *src && edge.destination == *tgt {
                            assert_eq!((-45., 25.), edge.via_point);
                            return;
                        }
                    }
                }
                panic!("State did not contain edge");
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
        send_player_action(&gs, 1, PlayerAction::StartGame);
        tick(&ticks);
        assert_is_game_start(&pr1.recv().unwrap());
        assert_is_game_start(&pr2.recv().unwrap());
        let attemptSrc = StationId(0);
        let attemptTgt = StationId(1);
        send_player_action(&gs, 1, PlayerAction::ConnectStations(attemptSrc.clone(), attemptTgt.clone()));
        tick(&ticks);
        assert_has_edge(&pr1.recv().unwrap(), &attemptSrc, &attemptTgt);
        assert_has_edge(&pr2.recv().unwrap(), &attemptSrc, &attemptTgt);
    }
}
