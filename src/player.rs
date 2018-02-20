use std::net::{TcpListener, TcpStream, IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use serde_json;
use tungstenite;
use tungstenite::{WebSocket, Message};
use tungstenite::error::Error;
use tungstenite::protocol::Role;

use events::{InputEvent, StateUpdate};
use player_id::*;
use sexpect::*;

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

pub fn connection_handler(new_player_sender: Sender<Box<WebSocket<TcpStream>>>, listener: TcpListener, murder_host: String) {
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

pub fn player_handler(
    player_receiver: Receiver<Box<WebSocket<TcpStream>>>,
    to_server: Sender<InputEvent>,
) {
    let mut next_id = 0u16;
    for mut player in player_receiver.iter() {
        println!("New Player {:?}!", next_id);
        let (to_player_s, to_player_r) = channel();
        let id = PlayerId::new(next_id);
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
