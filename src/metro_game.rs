use std::collections::{ HashMap, HashSet };
use std::sync::mpsc::Receiver;

use rand::{Rng, thread_rng};

use events::{ InputEvent };
use game::Game;
use player_id::*;
use player::Player;
use ticks::*;
use randoms::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum PlayerAction {
    StartGame,
    NewLine(StationId, StationId),
    InsertAtLineBeginning(LineId, StationId),
    InsertAtLineEnd(LineId, StationId),
    InsertBetweenStations(LineId, StationId, StationId, StationId),
}

#[derive(Debug, PartialEq, Serialize)]
pub enum StateUpdate {
    LobbyCount(u8),
    GameState(MetroModel),
    You(PlayerId),
}

// This would probably be better off with state-handling trait and types
#[derive(Debug, Eq, PartialEq)]
enum MGameState {
    Lobby,
    Game,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Hash)]
pub enum StationType {
    Circle,
    Triangle,
    Square,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Hash)]
pub struct StationId(pub usize);

pub type Point = (f32, f32);

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Station {
    t: StationType,
    position: Point,
    passengers: Vec<StationType>,
    blow_time: u32,
}

impl Station {
    pub fn new(t: StationType, position: Point) -> Self {
        Self {
            t: t,
            position: position,
            passengers: Vec::new(),
            blow_time: 0u32,
        }
    }
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
    EndOfTheLine(&'a Edge),
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
                return GetEdgeResult::EndOfTheLine(last_edge);
            }
        }
        GetEdgeResult::LocNotFound
    }
    fn get_edge_before_station(&self, id: &StationId) -> GetEdgeResult {
        for e in self.edges.iter() {
            if e.destination == *id {
                return GetEdgeResult::Edge(e);
            }
        }
        if let Some(first_edge) = self.edges.first() {
            if first_edge.origin == *id {
                return GetEdgeResult::EndOfTheLine(first_edge);
            }
        }
        GetEdgeResult::LocNotFound
    }

    fn is_valid_to_add_station(&self, station_id: &StationId) -> bool {
        if self.edges.len() == 0 { return false; }
        if self.edges[0].origin == self.edges[self.edges.len() - 1].destination { return false; }
        for e in self.edges.iter() {
            if e.origin == *station_id {
                return false;
            }
        }
        true
    }

    fn all_stations(&self) -> Vec<&StationId> {
        if self.edges.len() == 0 { return Vec::new() }
        let mut v = Vec::new();
        v.push(&self.edges[0].origin);
        for e in self.edges.iter() {
            v.push(&e.destination);
        }
        v
    }

    fn stations_after(&self, station_id: &StationId, forward: bool) -> Vec<&StationId> {
        let mut all = self.all_stations();
        if !forward {
            all.reverse();
        }
        if all.len() == 0 { return all; }
        let mut station_index = 0;
        for s in all.iter() {
            if s == &station_id {
                break;
            }
            station_index += 1;
        }
        if station_index >= all.len() {
            return Vec::new();
        }
        if all[0] == all[all.len() - 1] {
            all.rotate_left(station_index);
            return all;
        } 
        if station_index == all.len() - 1 {
            all.reverse();
            return all[1..].into();
        }
        return all[station_index+1..].into();
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
    passengers: Vec<StationType>,
    passenger_wait: Option<u16>,
}

impl Train {
    pub fn new(line_id: LineId, pos: Point, via: Point, forward: bool, origin: StationId, next: StationId, speed: f32) -> Self {
        Self {
            on_line: line_id,
            position: pos,
            heading: via,
            forward: forward,
            between_stations: (origin, next),
            speed: speed,
            passengers: Vec::new(),
            passenger_wait: None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum TrainNextTarget {
    Heading(Point),
    Reverse(Point),
    Edge(StationId, Point, StationId),
    None,
}

enum PassengerAction {
    Destination,
    Change,
    Boarding,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct MetroModel {
    stations: Vec<Station>,
    lines: Vec<Line>,
    trains: Vec<Train>,
    station_size: u8,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    time_to_blow: u32,
    scores: HashMap<PlayerId, u16>,
}

impl MetroModel {
    pub fn new() -> Self {
        Self {
            stations: Vec::new(),
            station_size: 26u8,
            lines: Vec::new(),
            trains: Vec::new(),
            min_x: -500.,
            min_y: -500.,
            max_x: 500.,
            max_y: 500.,
            time_to_blow: 1350u32,
            scores: HashMap::new(),
        }
    }
    pub fn get_station(&self, id: &StationId) -> Option<&Station> {
        let &StationId(index) = id;
        self.stations.get(index)
    }

    pub fn get_station_mut(&mut self, id: &StationId) -> Option<&mut Station> {
        let &StationId(index) = id;
        self.stations.get_mut(index)
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

    pub fn get_line_mut(&mut self, id: &LineId) -> Option<&mut Line> {
        let &LineId(index) = id;
        self.lines.get_mut(index)
    }

    pub fn add_train_to_line(&mut self, id: &LineId) {
        let &LineId(index) = id;
        let line = self.lines.get(index);
        let (station1, station2, via) = if let Some(line) = line {
            (line.edges[0].origin.clone(), line.edges[0].destination.clone(), line.edges[0].via_point)
        } else {
            return
        };
        let train = Train::new(id.clone(), self.get_station_pos(&station1).unwrap_or((0f32,0f32)), via, true, station1, station2, 1.);
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
        if let Some(_wait) = train.passenger_wait {
            return;
        }
        let point_proximity = train.speed;
        let dx = train.heading.0 - train.position.0;
        let dy = train.heading.1 - train.position.1;
        let sqr_dist = dx * dx + dy * dy;
        let dist = sqr_dist.sqrt();
        if dist > point_proximity {
            // They go faster diagonally...?
            if dx != 0. {
                train.position.0 += dx.signum() * train.speed;                
            }
            if dy != 0. {
                train.position.1 += dy.signum() * train.speed;
            }
        } else if dist > 0. {
            train.position = train.heading.clone();
        } 
    }

    fn train_reached_destination(&self, id: &TrainId) -> bool {
        match self.get_train(id) {
            Some(t) => t.position == t.heading,
            None => false,
        }
    }

    fn get_train_next_destination(&self, id: &TrainId) -> TrainNextTarget {
        if let Some(t) = self.get_train(id) {
            if t.position != t.heading {
                return TrainNextTarget::Heading(t.heading);
            }
            let target_id = if t.forward { &t.between_stations.1 } else { &t.between_stations.0 };
            if let Some(target_pos) = self.get_station_pos(target_id) {
                if t.heading != target_pos {
                    return TrainNextTarget::Heading(target_pos);
                }
                if let Some(line) = self.get_line(&t.on_line) {
                    let next_line = if t.forward { line.get_edge_after_station(target_id) } else { line.get_edge_before_station(target_id) };
                    match next_line {
                        GetEdgeResult::Edge(e) =>
                            return TrainNextTarget::Edge(e.origin.clone(), e.via_point, e.destination.clone()),
                        GetEdgeResult::LocNotFound =>
                            return TrainNextTarget::None,
                        GetEdgeResult::EndOfTheLine(e) =>
                            return TrainNextTarget::Reverse(e.via_point),
                    }
                }
            }
        }
        TrainNextTarget::None
    }

    fn get_at_station(&self, id: &TrainId) -> Option<StationId> {
        self.get_train(id)
            .and_then(|t| {
                let pos = t.position;
                let (ref s1, ref s2) = t.between_stations;
                  self.get_station(&s1)
                      .and_then(|s|
                                if pos == s.position {
                                    Some(s1.clone())
                                } else {
                                    None
                                }
                            )
                      .or(self.get_station(&s2)
                          .and_then(|s|
                                if pos == s.position {
                                    Some(s2.clone())
                                } else {
                                    None
                                }
                            )
                          )
            })
    }

    fn passengers_who_want_to_alight(&self, train: &Train, station_id: &StationId) -> Vec<StationType> {
        let mut on_train = HashSet::with_capacity(6);
        for p in train.passengers.iter() {
            on_train.insert(p);
        }

        let mut deliverable = HashSet::new();

        if let Some(s) = self.get_station(station_id) {
            if on_train.remove(&s.t) {
                deliverable.insert(s.t.clone());
            }
        }

        let line_id = &train.on_line;
        if let Some(line) = self.get_line(line_id) {
            let stations_after = line.stations_after(station_id, train.forward);
            for s_id in stations_after.iter() {
                if let Some(station) = self.get_station(s_id) {
                    // These guys are committed to this train
                    on_train.remove(&station.t);
                }
            }

            for (other_line_index, other_line) in self.lines.iter().enumerate() {
                if LineId(other_line_index) == *line_id {
                    continue;
                }
                let other_stations = other_line.all_stations();
                if !other_stations.contains(&station_id) {
                    continue;
                }
                for other_s_id in other_stations.iter() {
                    if let Some(station) = self.get_station(other_s_id) {
                        if on_train.remove(&station.t) {
                            deliverable.insert(station.t.clone());
                        }
                    }
                }
                if on_train.is_empty() {
                    break;
                }
            }
        }
        deliverable.iter().cloned().collect()
    }

    fn passengers_who_want_to_board(&self, train: &Train, station_id: &StationId) -> Vec<StationType> {
        let mut at_station = HashSet::with_capacity(6);
        if let Some(station) = self.get_station(station_id) {
            for p in station.passengers.iter() {
                at_station.insert(p);
            }
        }

        let mut deliverable = HashSet::new();
        let line_id = &train.on_line;
        if let Some(line) = self.get_line(line_id) {
            // Check each station on line for passenger drop off
            // Then check lines from those to other drop offs
            // Stop if we empty at_station
            let stations_after = line.stations_after(station_id, train.forward);
            for s_id in stations_after.iter() {
                if let Some(station) = self.get_station(s_id) {
                    if at_station.remove(&station.t) {
                        deliverable.insert(station.t.clone());
                    }
                }
                if at_station.is_empty() {
                    break;
                }
            }
            for s_id in stations_after.iter() {
                for (other_line_index, other_line) in self.lines.iter().enumerate() {
                    if LineId(other_line_index) == *line_id {
                        continue;
                    }
                    let other_stations = other_line.all_stations();
                    if !other_stations.contains(s_id) {
                        continue;
                    }
                    for other_s_id in other_stations.iter() {
                        if let Some(station) = self.get_station(other_s_id) {
                            if at_station.remove(&station.t) {
                                deliverable.insert(station.t.clone());
                            }
                        }
                        if at_station.is_empty() {
                            break;
                        }
                    }
                    if at_station.is_empty() {
                        break;
                    }

                }
                if at_station.is_empty() {
                    break;
                }
            }
        }

        deliverable.iter().cloned().collect()
    }

    fn station_passenger(&self, id: &TrainId) -> Option<(StationType, PassengerAction)> {
        self.get_train(id)
            .and_then(|t| {
                self.get_at_station(id)
                    .map(|s_id| (t, s_id))
            })
            .and_then(|(t, s_id)| {
                self.get_station(&s_id)
                    .and_then(|s| {
                        if t.position != s.position {
                            return None;
                        }
                        if t.passengers.contains(&s.t) {
                            return Some((s.t.clone(), PassengerAction::Destination))
                        }
                        if t.passengers.len() > 0 {
                            if let Some(p) = self.passengers_who_want_to_alight(&t, &s_id).first() {
                                return Some((p.clone(), if p == &s.t { PassengerAction::Destination } else { PassengerAction::Change }));
                            }
                        }
                        if t.passengers.len() < 6 {
                            let boarding = self.passengers_who_want_to_board(&t, &s_id);
                            return boarding.first().map(|t| (t.clone(), PassengerAction::Boarding));
                        }
                        None
                    })
            })
    }

    fn handle_train_arrival(&mut self, id: &TrainId) {
        self.get_train_mut(id).map(|t| t.passenger_wait = t.passenger_wait.map(|w| w - 1));
        let t_pass = self.get_train(id).map(|t| t.passenger_wait);
        let can_transfer = t_pass == Some(Some(0));
        let can_start_new = can_transfer || t_pass == Some(None);
        if can_transfer {
            match self.station_passenger(id) {
                Some((passenger, PassengerAction::Destination)) =>  {
                    self.get_train_mut(id)
                        .map(|t: &mut Train| remove_first(&mut t.passengers, &passenger));
                    let owning_player = self.get_train(id).and_then(|t| self.get_line(&t.on_line)).map(|l| l.owning_player);
                    if let Some(p) = owning_player {
                        *self.scores.entry(p).or_insert(0) += 1;
                    }
                }
                Some((passenger, PassengerAction::Boarding)) =>  {
                    self.get_at_station(id)
                        .and_then(|s_id| self.get_station_mut(&s_id))
                        .map(|s| remove_first(&mut s.passengers, &passenger));
                    self.get_train_mut(id)
                        .map(|t: &mut Train| t.passengers.push(passenger));
                }
                Some((passenger, PassengerAction::Change)) =>  {
                    self.get_train_mut(id)
                        .map(|t: &mut Train| remove_first(&mut t.passengers, &passenger));
                    self.get_at_station(id)
                        .and_then(|s_id| self.get_station_mut(&s_id))
                        .map(|s| s.passengers.push(passenger));
                }
                None => {}
            }
        }
        if can_start_new {
            match self.station_passenger(id) {
                Some(_) => 
                    self.get_train_mut(id)
                        .map(|t: &mut Train| t.passenger_wait = Some(30)),
                None =>
                    self.get_train_mut(id)
                        .map(|t: &mut Train| t.passenger_wait = None),
            };
        }
    }

    fn handle_train_destination(&mut self, id: &TrainId) {
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
                TrainNextTarget::None => { },
            }
        }
    }

    fn update_train(&mut self, id : &TrainId) {
        self.step_train(id);
        self.handle_train_arrival(id);
        self.handle_train_destination(id);
    }

    fn update_station(&mut self, id: &StationId) {
        self.get_station_mut(id)
            .map(|s| {
                if s.passengers.len() > 12 {
                    s.blow_time += 1;
                } else {
                    s.blow_time = 0;
                }
            });
    }

    pub fn update(&mut self) {
        for i in 0..self.stations.len() {
            let id = StationId(i);
            self.update_station(&id);
        }
        for i in 0..self.trains.len() {
            let id = TrainId(i);
            self.update_train(&id);
        }
    }

    fn get_player_unused_line_id(&self, player: &PlayerId) -> Option<LineId> {
        for i in 0..self.lines.len() {
            if self.lines[i].edges.len() > 0 { continue; }
            if self.lines[i].owning_player == *player {
                return Some(LineId(i));
            }
        }
        None
    }

    fn get_via_point_between(&self, origin: &StationId, destination: &StationId) -> Point {
        let (origin_x, origin_y) = self.get_station(origin).map(|s| s.position).unwrap_or((0f32, 0f32));
        let (dest_x, dest_y) = self.get_station(destination).map(|s| s.position).unwrap_or_default();
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

    pub fn start_new_line(&mut self, player: &PlayerId, origin: &StationId, dest: &StationId) -> Option<LineId> {
        let via = self.get_via_point_between(origin, dest);
        if let Some(line_id) = self.get_player_unused_line_id(player) {
            if let Some(line) = self.get_line_mut(&line_id) {
                line.edges.push(Edge { origin: origin.clone(), destination: dest.clone(), via_point: via.clone() });
            }
            return Some(line_id);
        }
        None
    }

    pub fn insert_before_line(&mut self, line_id: &LineId, new_station: &StationId) {
        let line_origin_if_valid =
            if let Some(line) = self.get_line(&line_id) {
                if line.is_valid_to_add_station(new_station) {
                    line.edges[0].origin.clone()
                } else { return; }
            } else { return; };
        let via = self.get_via_point_between(new_station, &line_origin_if_valid);
        if let Some(line) = self.get_line_mut(&line_id) {
            line.edges.insert(0, Edge { origin: new_station.clone(), destination: line_origin_if_valid, via_point: via.clone() });
        }
    }

    pub fn insert_after_line(&mut self, line_id: &LineId, new_station: &StationId) {
        let line_dest_if_valid =
            if let Some(line) = self.get_line(&line_id) {
                if line.is_valid_to_add_station(new_station) {
                    line.edges[line.edges.len() - 1].destination.clone()
                } else { return; }
            } else { return; };
        let via = self.get_via_point_between(&line_dest_if_valid, new_station);
        if let Some(line) = self.get_line_mut(&line_id) {
            line.edges.push(Edge { origin: line_dest_if_valid, destination: new_station.clone(), via_point: via.clone() });
        }
    }

    pub fn is_valid_station_pos(&self, pos: &Point) -> bool {
        let (ref x, ref y) = pos;
        if x < &self.min_x || x > &self.max_x {
            return false;
        }
        if y < &self.min_y || y > &self.max_y {
            return false;
        }
        let max_square_distance = ((self.station_size + self.station_size) as f32).powi(2);
        for existing_station in self.stations.iter() {
            let (ref other_x, ref other_y) = existing_station.position;
            let diff_x = x - other_x;
            let diff_y = y - other_y;
            let square_distance = diff_x.powi(2) + diff_y.powi(2);
            if square_distance < max_square_distance {
                return false;
            }
        }
        true
    }
}

fn remove_first<T: Eq>(v: &mut Vec<T>, t: &T) {
    let mut delete_idx = None;
    for i in 0..(v.len()) {
        if v[i] == *t {
            delete_idx = Some(i);
            break;
        }
    }
    delete_idx.map(|i| v.remove(i));
}

pub struct MetroGame<T: Ticker, R: Random> {
    state: MGameState,
    r: Receiver<InputEvent>,
    player_out: HashMap<PlayerId, Player>,
    ticker: T,
    model: MetroModel,
    random: R,

    ticks_since_last_station: u64,
    min_ticks_between_stations: u64,
    base_station_chance: f64,
    station_chance_per_tick: f64,

    ticks_since_last_passenger: Vec<u64>,
    min_ticks_between_passengers: u64,
    base_passenger_chance: f64,
    passenger_chance_per_tick: f64,

    ticks_per_week: u64,
    ticks_since_weekend: u64,

    max_lines_per_player: u8,
}

impl<T: Ticker, R: Random> Game<T, R> for MetroGame<T, R> {
    fn new(event_loop: Receiver<InputEvent>, ticker: T, random: R) -> Self {
        MetroGame {
            state: MGameState::Lobby,
            r: event_loop,
            player_out: HashMap::new(),
            ticker: ticker,
            model: MetroModel::new(),
            random: random,

            ticks_since_last_station: 0,
            min_ticks_between_stations: 30,
            base_station_chance: 0.00005,
            station_chance_per_tick: 0.000005,

            ticks_since_last_passenger: Vec::new(),
            min_ticks_between_passengers: 30,
            base_passenger_chance: 0.00005,
            passenger_chance_per_tick: 0.000005,

            ticks_per_week: 4200,
            ticks_since_weekend: 0,

            max_lines_per_player: 7,
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

impl<T: Ticker, R: Random> MetroGame<T, R> {
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
                p.send_message(StateUpdate::You(p_id.clone()));
                self.player_out.insert(p_id, p);
            }
            InputEvent::Disconnection(p_id) => {
                self.player_out.remove(&p_id);
            }
            InputEvent::PlayerAction(p_id, action) => { 
                match action {
                    PlayerAction::NewLine(src, tgt) => {
                        let new_id = self.model.start_new_line(&p_id, &src, &tgt);
                        if let Some(new_id) = new_id {
                            self.model.add_train_to_line(&new_id);
                        }

                    }
                    PlayerAction::InsertAtLineBeginning(line_id, station_id) => {
                        self.model.insert_before_line(&line_id, &station_id);
                    }
                    PlayerAction::InsertAtLineEnd(line_id, station_id) => {
                        self.model.insert_after_line(&line_id, &station_id);
                    }
                    PlayerAction::InsertBetweenStations(_, _, _, _) => {}
                    PlayerAction::StartGame => {
                        // Game is already started
                    }
                }
            }
        }
    }

    fn get_player_ids(&self) -> Vec<PlayerId> {
        self.player_out.keys().cloned().collect()
    }
    
    fn handle_lobby_event(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::Connection(p_id, p) => {
                p.send_message(StateUpdate::You(p_id.clone()));
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
                        self.model.stations.push(Station::new(StationType::Circle, (10., -30.)));
                        for _ in 0..15 {
                            self.model.stations[0].passengers.push(StationType::Circle);
                        }
                        self.model.stations.push(Station::new(StationType::Square, (-45., 70.)));
                        self.model.stations.push(Station::new(StationType::Triangle, (300., 30.)));
                        for player in self.get_player_ids() {
                            self.add_line_for_player(&player);
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

    fn add_line_for_player(&mut self, player_id: &PlayerId) {
        let mut rng = thread_rng();
        let new_line = Line { edges: Vec::new(), colour: (rng.gen(), rng.gen(), rng.gen()), owning_player: player_id.clone() };
        self.model.lines.push(new_line);
    }

    fn finish_week(&mut self) {
        println!("Le weekend");
        for player in self.get_player_ids() {
            let mut player_line_count = 0;
            for line in self.model.lines.iter() {
                if line.owning_player == player {
                    player_line_count += 1;
                }
            }
            if player_line_count < self.max_lines_per_player {
                self.add_line_for_player(&player);
            }
        }
        self.ticks_since_weekend = 0;
    }

    fn update_week(&mut self) {
        self.ticks_since_weekend += 1;
        if self.ticks_since_weekend >= self.ticks_per_week {
            self.finish_week();
        }
    }

    fn random_station_type(&self) -> StationType {
        let roll = self.random.gen();
        if roll < 0.4 {
            StationType::Circle
        } else if roll < 0.7 {
            StationType::Square
        } else {
            StationType::Triangle
        }
    }
    pub fn update(&mut self) {
        if let Some(spawnable_ticks) = self.ticks_since_last_station.checked_sub(self.min_ticks_between_stations) {
            let chance = self.base_station_chance + self.station_chance_per_tick * spawnable_ticks as f64;
            if self.random.gen() < chance {
                let width = self.model.max_x - self.model.min_x;
                let height = self.model.max_y - self.model.min_y;
                let x = self.random.gen() as f32 * width + self.model.min_x;
                let y = self.random.gen() as f32 * height + self.model.min_y;
                if self.model.is_valid_station_pos(&(x, y)) {
                    let station_type = self.random_station_type();
                    self.model.stations.push(Station::new(station_type, (x, y)));
                }
                self.ticks_since_last_station = 0;
            }
        }
        self.ticks_since_last_station += 1;
        self.ticks_since_last_passenger.resize(self.model.stations.len(), 0);
        for i in 0..self.model.stations.len() {
            if let Some(spawnable_ticks) = self.ticks_since_last_passenger[i].checked_sub(self.min_ticks_between_passengers) {
                let chance = self.base_passenger_chance + self.passenger_chance_per_tick * spawnable_ticks as f64;
                if self.random.gen() < chance {
                    let station_type = self.random_station_type();
                    let station = StationId(i);
                    self.model.get_station_mut(&station)
                        .map(|s| 
                             if s.t != station_type {
                                 s.passengers.push(station_type)
                             });
                    self.ticks_since_last_passenger[i] = 0;
                }
            }
            self.ticks_since_last_passenger[i] += 1;
        }
        self.update_week();
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

    fn start_test_game() -> (Sender<InputEvent>, (Sender<f64>, Receiver<()>)) {
        let (tsw, trw) = channel();
        let (tss, trs) = channel();
        let t = TestTicker { r: trw, s: tss };
        let (gs, gr) = channel();
        let mut test_game = MetroGame::new(gr, t, Always1Random);
        thread::spawn(move || test_game.main());
        thread::sleep(Duration::from_millis(200));
        (gs, (tsw, trs))
    }

    fn send_player_action(sender: &Sender<InputEvent>, id: u16, action: PlayerAction) {
        sender.send(InputEvent::PlayerAction(PlayerId::new(id), action))
            .expect("Test sending player action");
    }

    fn tick(&(ref tsw, ref trs): &(Sender<f64>, Receiver<()>)) {
        tsw.send(1f64).unwrap();
        println!("waiting tick end");
        trs.recv().unwrap();
        println!("tick end");
    }

    #[test]
    fn connecting_players() {
        let (gs, ticks) = start_test_game();
        let pr1 = connect_player(&gs, 1);
        tick(&ticks);
        assert_eq!(Ok(StateUpdate::You(PlayerId::new(1))), pr1.recv());
        assert_eq!(Ok(StateUpdate::LobbyCount(1)), pr1.recv());
        let pr2 = connect_player(&gs, 2);
        tick(&ticks);
        assert_eq!(Ok(StateUpdate::LobbyCount(2)), pr1.recv());
        assert_eq!(Ok(StateUpdate::You(PlayerId::new(2))), pr2.recv());
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
    fn assert_has_edge(update: &StateUpdate, src: &StationId, tgt: &StationId, via: Option<Point>) {
        match *update {
            StateUpdate::GameState(ref state) => {
                for line in state.lines.iter() {
                    for edge in line.edges.iter() {
                        if edge.origin == *src && edge.destination == *tgt {
                            if let Some(via) = via {
                                assert_eq!(via, edge.via_point);
                            }
                            return;
                        }
                    }
                }
                panic!("State did not contain edge: {:?}", state);
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
        assert_eq!(Ok(StateUpdate::You(PlayerId::new(1))), pr1.recv());
        assert_eq!(Ok(StateUpdate::You(PlayerId::new(2))), pr2.recv());
        assert_is_game_start(&pr1.recv().unwrap());
        assert_is_game_start(&pr2.recv().unwrap());
        let attempt_src = StationId(0);
        let attempt_tgt = StationId(1);
        send_player_action(&gs, 1, PlayerAction::NewLine(attempt_src.clone(), attempt_tgt.clone()));
        tick(&ticks);
        assert_has_edge(&pr1.recv().unwrap(), &attempt_src, &attempt_tgt, Some((-45., 25.)));
        assert_has_edge(&pr2.recv().unwrap(), &attempt_src, &attempt_tgt, Some((-45., 25.)));
    }

    #[test]
    fn insert_before_line() {
        let (gs, ticks) = start_test_game();
        let pr1 = connect_player(&gs, 1);
        send_player_action(&gs, 1, PlayerAction::StartGame);
        tick(&ticks);
        assert_eq!(Ok(StateUpdate::You(PlayerId::new(1))), pr1.recv());
        pr1.recv().unwrap();
        send_player_action(&gs, 1, PlayerAction::NewLine(StationId(0), StationId(1)));
        tick(&ticks);
        pr1.recv().unwrap();
        send_player_action(&gs, 1, PlayerAction::InsertAtLineBeginning(LineId(0), StationId(2)));
        tick(&ticks);
        assert_has_edge(&pr1.recv().unwrap(), &StationId(2), &StationId(0), None);
    }

    #[test]
    fn train_dest_choice_along_single_edge() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();

        let test_origin = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        let test_dest = Station::new (
            StationType::Circle,
            (10., 20.),
        );

        let test_edge = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (10., 10.),
        };

        m.stations.push(test_origin);
        m.stations.push(test_dest);
        m.lines.push(Line { edges: vec![ test_edge ], colour: (0., 0., 0.), owning_player: player });
        m.trains.push(Train::new(LineId(0), (0., 0.), (10., 10.), true, StationId(0), StationId(1), 1.));

        assert_eq!(TrainNextTarget::Heading((10., 10.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (5., 5.);
        assert_eq!(TrainNextTarget::Heading((10., 10.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (10., 10.);
        assert_eq!(TrainNextTarget::Heading((10., 20.)), m.get_train_next_destination(&TrainId(0)));
        m.trains[0].heading = (10., 20.);
        assert_eq!(TrainNextTarget::Heading((10., 20.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (10., 15.);
        assert_eq!(TrainNextTarget::Heading((10., 20.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (10., 20.);
        assert_eq!(TrainNextTarget::Reverse((10., 10.)), m.get_train_next_destination(&TrainId(0)));
        m.trains[0].heading = (10., 10.);
        m.trains[0].forward = false;
        assert_eq!(TrainNextTarget::Heading((10., 10.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (10., 15.);
        assert_eq!(TrainNextTarget::Heading((10., 10.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (10., 10.);
        assert_eq!(TrainNextTarget::Heading((0., 0.)), m.get_train_next_destination(&TrainId(0)));
        m.trains[0].heading = (0., 0.);
        assert_eq!(TrainNextTarget::Heading((0., 0.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (5., 5.);
        assert_eq!(TrainNextTarget::Heading((0., 0.)), m.get_train_next_destination(&TrainId(0)));

        m.trains[0].position = (0., 0.);
        assert_eq!(TrainNextTarget::Reverse((10., 10.)), m.get_train_next_destination(&TrainId(0)));
    }

    #[test]
    fn train_updating() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();

        let test_origin = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        let test_dest = Station::new (
            StationType::Circle,
            (10., 20.),
        );

        let test_edge = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (10., 10.),
        };

        m.stations.push(test_origin);
        m.stations.push(test_dest);
        m.lines.push(Line { edges: vec![ test_edge ], colour: (0., 0., 0.), owning_player: player });
        m.trains.push(Train::new(LineId(0), (0., 0.), (10., 10.), true, StationId(0), StationId(1), 5.));

        // Just assert I'm not crazy
        assert_eq!((0., 0.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((5., 5.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((10., 10.), m.trains[0].position);
        assert_eq!((10., 20.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((10., 15.), m.trains[0].position);
        assert_eq!((10., 20.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((10., 20.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((10., 15.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((10., 10.), m.trains[0].position);
        assert_eq!((0., 0.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((5., 5.), m.trains[0].position);
        assert_eq!((0., 0.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

        m.update_train(&TrainId(0));
        assert_eq!((0., 0.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
    }

    #[test]
    pub fn test_drifting() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();

        let test_loc1 = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        let test_loc2 = Station::new (
            StationType::Circle,
            (10., 20.),
        );
        let test_loc3 = Station::new (
            StationType::Circle,
            (30., 10.),
        );

        let test_edge1 = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (10., 10.),
        };
        let test_edge2 = Edge {
            origin: StationId(1),
            destination: StationId(2),
            via_point: (20., 10.),
        };

        m.stations.push(test_loc1);
        m.stations.push(test_loc2);
        m.stations.push(test_loc3);
        m.lines.push(Line { edges: vec![ test_edge1, test_edge2 ], colour: (0., 0., 0.), owning_player: player });
        m.trains.push(Train::new(LineId(0), (0., 0.), (10., 10.), true, StationId(0), StationId(1), 5.));

        m.update_train(&TrainId(0));
        assert_eq!((5., 5.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((10., 10.), m.trains[0].position);
        assert_eq!((10., 20.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((10., 15.), m.trains[0].position);
        assert_eq!((10., 20.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((10., 20.), m.trains[0].position);
        assert_eq!((20., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((15., 15.), m.trains[0].position);
        assert_eq!((20., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((20., 10.), m.trains[0].position);
        assert_eq!((30., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((25., 10.), m.trains[0].position);
        assert_eq!((30., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((30., 10.), m.trains[0].position);
        assert_eq!((20., 10.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((25., 10.), m.trains[0].position);
        assert_eq!((20., 10.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((20., 10.), m.trains[0].position);
        assert_eq!((10., 20.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((15., 15.), m.trains[0].position);
        assert_eq!((10., 20.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((10., 20.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((10., 15.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((10., 10.), m.trains[0].position);
        assert_eq!((0., 0.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((5., 5.), m.trains[0].position);
        assert_eq!((0., 0.), m.trains[0].heading);
        assert_eq!(false, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        m.update_train(&TrainId(0));
        assert_eq!((0., 0.), m.trains[0].position);
        assert_eq!((10., 10.), m.trains[0].heading);
        assert_eq!(true, m.trains[0].forward);
        assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
    }

    #[test]
    pub fn line_loop() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();

        let test_loc1 = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        let test_loc2 = Station::new (
            StationType::Circle,
            (10., 20.),
        );
        let test_loc3 = Station::new (
            StationType::Circle,
            (30., 10.),
        );

        let test_edge1 = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (10., 10.),
        };
        let test_edge2 = Edge {
            origin: StationId(1),
            destination: StationId(2),
            via_point: (20., 10.),
        };
        let test_edge3 = Edge {
            origin: StationId(2),
            destination: StationId(0),
            via_point: (20., 0.),
        };

        m.stations.push(test_loc1);
        m.stations.push(test_loc2);
        m.stations.push(test_loc3);
        m.lines.push(Line { edges: vec![ test_edge1, test_edge2, test_edge3 ], colour: (0., 0., 0.), owning_player: player });
        m.trains.push(Train::new(LineId(0), (0., 0.), (10., 10.), true, StationId(0), StationId(1), 10.));

        // It's a loop so we should be able to do the same thing in a loop
        for _ in 0..5 {
            m.update();
            assert_eq!((10., 10.), m.trains[0].position);
            assert_eq!((10., 20.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);

            m.update();
            assert_eq!((10., 20.), m.trains[0].position);
            assert_eq!((20., 10.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);

            m.update();
            assert_eq!((20., 10.), m.trains[0].position);
            assert_eq!((30., 10.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(1), StationId(2)), m.trains[0].between_stations);

            m.update();
            assert_eq!((30., 10.), m.trains[0].position);
            assert_eq!((20., 0.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(2), StationId(0)), m.trains[0].between_stations);

            m.update();
            assert_eq!((20., 0.), m.trains[0].position);
            assert_eq!((0., 0.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(2), StationId(0)), m.trains[0].between_stations);

            m.update();
            assert_eq!((10., 0.), m.trains[0].position);
            assert_eq!((0., 0.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(2), StationId(0)), m.trains[0].between_stations);

            m.update();
            assert_eq!((0., 0.), m.trains[0].position);
            assert_eq!((10., 10.), m.trains[0].heading);
            assert_eq!(true, m.trains[0].forward);
            assert_eq!((StationId(0), StationId(1)), m.trains[0].between_stations);
        }
    }

    #[test]
    pub fn passenger_alighting() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();

        let test_loc1 = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        let test_loc2 = Station::new (
            StationType::Triangle,
            (10., 20.),
        );

        let test_edge1 = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (10., 10.),
        };
        m.stations.push(test_loc1);
        m.stations.push(test_loc2);
        m.lines.push(Line { edges: vec![ test_edge1 ], colour: (0., 0., 0.), owning_player: player });
        let mut train = Train::new(LineId(0), (0., 0.), (10., 10.), true, StationId(0), StationId(1), 10.);
        train.passengers.push(StationType::Circle);
        train.passengers.push(StationType::Triangle);
        train.passengers.push(StationType::Square);
        train.passengers.push(StationType::Circle);
        m.trains.push(train);

        m.update();
        assert_eq!((10., 10.), m.trains[0].position);

        m.update();
        assert_eq!((10., 20.), m.trains[0].position);

        // Start waiting to deposit passengers.

        for _ in 0..30 {
            assert_eq!((10., 20.), m.trains[0].position);
            assert_eq!(vec![StationType::Circle, StationType::Triangle, StationType::Square, StationType::Circle], m.trains[0].passengers);
            m.update();
        }
        assert_eq!((10., 20.), m.trains[0].position);
        assert_eq!(vec![StationType::Circle, StationType::Square, StationType::Circle], m.trains[0].passengers);

        m.update();
        assert_eq!((10., 10.), m.trains[0].position);

        m.update();
        assert_eq!((0., 0.), m.trains[0].position);

        for _ in 0..30 {
            assert_eq!((0., 0.), m.trains[0].position);
            assert_eq!(vec![StationType::Circle, StationType::Square, StationType::Circle], m.trains[0].passengers);
            m.update();
        }
        for _ in 0..30 {
            assert_eq!((0., 0.), m.trains[0].position);
            assert_eq!(vec![StationType::Square, StationType::Circle], m.trains[0].passengers);
            m.update();
        }
        assert_eq!((0., 0.), m.trains[0].position);
        assert_eq!(vec![StationType::Square], m.trains[0].passengers);

        m.update();
        assert_eq!((10., 10.), m.trains[0].position);
    }

    #[test]
    pub fn passengers_for_change() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();
        let mut test_loc1 = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        test_loc1.passengers.push(StationType::Triangle);
        test_loc1.passengers.push(StationType::Square);
        let mut test_loc2 = Station::new (
            StationType::Circle,
            (0., 10.),
        );
        test_loc2.passengers.push(StationType::Triangle);
        test_loc2.passengers.push(StationType::Square);
        let test_loc3 = Station::new (
            StationType::Triangle,
            (10., 0.),
        );
        m.stations.push(test_loc1);
        m.stations.push(test_loc2);
        m.stations.push(test_loc3);
        let test_edge1 = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (0., 5.),
        };
        let test_edge2 = Edge {
            origin: StationId(0),
            destination: StationId(2),
            via_point: (5., 0.),
        };
        m.lines.push(Line { edges: vec![ test_edge1 ], colour: (0., 0., 0.), owning_player: player });
        m.lines.push(Line { edges: vec![ test_edge2 ], colour: (0., 0., 0.), owning_player: player });

        let train = Train::new(LineId(0), (0., 10.), (0., 5.), true, StationId(0), StationId(1), 5.);
        m.trains.push(train);
        assert_eq!(vec![StationType::Triangle, StationType::Square], m.get_station(&StationId(1)).unwrap().passengers);
        assert_eq!(vec![StationType::Triangle], m.passengers_who_want_to_board(&m.trains[0], &StationId(1)));

        let train = Train::new(LineId(0), (0., 0.), (0., 5.), true, StationId(0), StationId(1), 5.);
        m.trains.push(train);
        assert_eq!(vec![StationType::Triangle, StationType::Square], m.get_station(&StationId(0)).unwrap().passengers);
        assert_eq!(Vec::<StationType>::new(), m.passengers_who_want_to_board(&m.trains[1], &StationId(0)));
    }

    #[test]
    pub fn line_stations_after() {
        let test_edge1 = Edge {
            origin: StationId(0),
            destination: StationId(1),
            via_point: (0., 5.),
        };
        let test_edge2 = Edge {
            origin: StationId(1),
            destination: StationId(2),
            via_point: (5., 0.),
        };
        let line = Line { edges: vec![ test_edge1, test_edge2 ], colour: (0., 0., 0.), owning_player: PlayerId::new(0) };
        assert_eq!(vec![&StationId(1), &StationId(2)], line.stations_after(&StationId(0), true));
        assert_eq!(vec![&StationId(2)], line.stations_after(&StationId(1), true));
        assert_eq!(vec![&StationId(1), &StationId(0)], line.stations_after(&StationId(2), true));
        assert_eq!(vec![&StationId(1), &StationId(0)], line.stations_after(&StationId(2), false));
        assert_eq!(vec![&StationId(0)], line.stations_after(&StationId(1), false));
        assert_eq!(vec![&StationId(1), &StationId(2)], line.stations_after(&StationId(0), false));
    }

    #[test]
    pub fn can_create_loop() {
        let player = PlayerId::new(0);
        let mut m = MetroModel::new();

        let test_loc1 = Station::new (
            StationType::Circle,
            (0., 0.),
        );
        let test_loc2 = Station::new (
            StationType::Triangle,
            (10., 20.),
        );
        let test_loc3 = Station::new (
            StationType::Triangle,
            (20., 0.),
        );
        m.stations.push(test_loc1);
        m.stations.push(test_loc2);
        m.stations.push(test_loc3);
        m.lines.push(Line { edges: vec![ ], colour: (0., 0., 0.), owning_player: player });

        m.start_new_line(&player, &StationId(0), &StationId(1));
        assert_eq!(1, m.lines[0].edges.len());
        m.insert_after_line(&LineId(0), &StationId(2));
        assert_eq!(2, m.lines[0].edges.len());
        m.insert_after_line(&LineId(0), &StationId(0));
        //Shouldn't fail
        //assert_eq!(3, m.lines[0].edges.len());
    }
}
