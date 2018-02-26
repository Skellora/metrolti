use player_id::*;
use player::Player;
use metro_game::PlayerAction;

#[derive(Debug)]
pub enum InputEvent {
    PlayerAction(PlayerId, PlayerAction),
    Connection(PlayerId, Player),
    Disconnection(PlayerId),
}

