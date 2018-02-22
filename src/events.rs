use player_id::*;
use player::Player;

#[derive(Debug)]
pub enum InputEvent {
    PlayerAction(PlayerId, PlayerAction),
    Connection(PlayerId, Player),
    Disconnection(PlayerId),
}

#[derive(Debug, Deserialize)]
pub enum PlayerAction {
    StartGame,
}

#[derive(Debug, Serialize)]
pub enum StateUpdate {
    LobbyCount(u8),
}

