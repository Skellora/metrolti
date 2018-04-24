extern crate iron;
extern crate mount;
extern crate staticfile;
extern crate handlebars;
extern crate handlebars_iron;
extern crate tungstenite;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate rand;
#[cfg(test)] 
#[macro_use]
extern crate pretty_assertions;

pub mod game;
pub mod metro_game;
mod events;
mod player_id;
mod player;
pub mod server;
mod sexpect;
pub mod web;
pub mod ticks;
pub mod randoms;
