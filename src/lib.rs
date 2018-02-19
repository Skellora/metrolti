extern crate tungstenite;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::fmt::Debug;
use std::net::{ IpAddr, Ipv4Addr };
use std::str::FromStr;

use tungstenite::{WebSocket, Message};
use tungstenite::protocol::Role;
use tungstenite::error::Error;

pub fn listen<G: Game>(websocket_address: String, murder_host: String) {
    let (connection_sender, connection_receiver) = channel();
    let (to_server_sender, to_server_receiver) = channel();
    thread::spawn(move || {
        let tcp = TcpListener::bind(websocket_address).unwrap();
        connection_handler(connection_sender, tcp, murder_host);
    });
    thread::spawn(move || {
        player_handler(connection_receiver, to_server_sender);
    });
    thread::spawn(move || {
        G::new(to_server_receiver).main();
    });
}

#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub struct PlayerId(u16);

fn handle_player_in(to_server: Sender<InputEvent>, in_stream: TcpStream, id: PlayerId) {
    let mut ws = WebSocket::from_raw_socket(in_stream, Role::Server);
    loop {
        match ws.read_message() {
            Ok(m) => {
                let m_text = m.into_text().expect("message into text");
                let json_m = serde_json::from_str(&m_text);
                match json_m {
                    Ok(player_m) => {
                        let translated_message = InputEvent::Message(id.clone(), player_m);
                        to_server.send(translated_message)
                          .sexpect("Failed to forward message to server");
                    }
                    Err(e) => {
                        println!("Failed to deserialize message ({:?}): {:?}", e, m_text);
                    }
                }
            }
            Err(Error::ConnectionClosed(_)) => {
                let _ = to_server.send(InputEvent::Disconnection(id));
                println!("{:?} disconnected", id);
                break;
            }
            _ => {}
        }
    }
    println!("Dropping {:?} in handler", id);
}

fn handle_player_out(from_server: Receiver<StateUpdate>, out_stream: TcpStream, id: PlayerId) {
    let mut ws = WebSocket::from_raw_socket(out_stream, Role::Server);
    for m in from_server.iter() {
        let serialized = serde_json::to_string(&m).expect("serualize");
        ws.write_message(Message::text(serialized))
          .sexpect(&format!("Failed to forward message to {:?}", id));
    }
    println!("Dropping {:?} out handler", id);
}

fn connection_handler(new_player_sender: Sender<Box<WebSocket<TcpStream>>>, listener: TcpListener, murder_host: String) {
    println!("listening");
    for incoming in listener.incoming() {
        let tcp_stream = incoming.expect("tcp stream");
        let murderable = 
            Ipv4Addr::from_str(&murder_host).map(IpAddr::V4) == 
            Ok(tcp_stream.peer_addr().unwrap().ip());
        let accepted = tungstenite::accept(tcp_stream);
        match accepted {
            Ok(ws) => {
                new_player_sender.send(Box::new(ws))
                  .sexpect("Error sending websocket to player handler");
            }
            Err(e) => {
                if murderable {
                    println!("quittin");
                    return;
                }
                println!("Failed websocket connection {:?}", e);
            }
        }
    }
}

fn player_handler(
    player_receiver: Receiver<Box<WebSocket<TcpStream>>>,
    to_server: Sender<InputEvent>,
) {
    let mut next_id = 0u16;
    for mut player in player_receiver.iter() {
        println!("New Player {:?}!", next_id);
        let (to_player_s, to_player_r) = channel();
        let id = PlayerId(next_id);
        match to_server.send(InputEvent::Connection(id, Player::new(to_player_s))) {
            Ok(_) => {
                let s = player.get_ref().try_clone().expect("stream cloning");
                let s2 = player.get_ref().try_clone().expect("stream cloning");
                let to_s = to_server.clone();
                let p_id = id.clone();
                let p_id2 = id.clone();
                thread::spawn(move || handle_player_in(to_s, s, p_id));
                thread::spawn(move || handle_player_out(to_player_r, s2, p_id2));
                next_id = next_id.wrapping_add(1);
            }
            Err(e) => {
                println!("Failed sending player to game: {:?}", e);
                let _ = player.write_message(Message::text("Connection failed"));
            }
        }
    }
}

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

#[derive(Debug)]
pub struct Player {
    sender: Sender<StateUpdate>,
}

impl Player {
    pub fn new(s: Sender<StateUpdate>) -> Self {
        Player { sender: s }
    }

    pub fn send_message(&self, message: StateUpdate) {
        self.sender.send(message)
          .sexpect("Failed to send message to player handler");
    }
}

pub trait Game {
    fn new(event_loop: Receiver<InputEvent>) -> Self;
    fn main(&mut self);
}

trait SoftExpect<E> {
    fn sexpect(self, message: &str);
}

impl<E> SoftExpect<E> for Result<(), E> 
  where E: Debug {
    fn sexpect(self, message: &str) {
        if let Err(e) = self {
            println!("{:?}: {:?}", message, e);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate url;

    use super::*;
    use std::time::{Duration};
    use self::url::Url;

    #[test]
    fn server_comms1() {
        listen("0.0.0.0:12345".to_string(), "127.0.0.1".to_string());

        let uri = Url::parse("ws://localhost:12345").unwrap();
        let mut ws = tungstenite::connect(uri).unwrap().0;
        ws.write_message(tungstenite::Message::text("test".to_string()));

        assert_eq!("test", ws.read_message().unwrap().to_text().unwrap());


        TcpStream::connect("localhost:12345").unwrap();
        thread::sleep(Duration::from_secs(1));

        assert!(TcpStream::connect("localhost:12345").is_err());
    }
}
