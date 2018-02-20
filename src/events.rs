use player_id::*;
use player::Player;

#[derive(Debug)]
pub enum InputEvent {
    Message(PlayerId, PlayerAction),
    Connection(PlayerId, Player),
    Disconnection(PlayerId),
}

#[derive(Debug, Deserialize)]
pub struct PlayerAction;

#[derive(Debug, Serialize)]
pub struct StateUpdate;

