extern crate tungstenite;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::fmt::Debug;
use std::net::{ IpAddr, Ipv4Addr };
use std::str::FromStr;

use tungstenite::WebSocket;

fn listen(websocket_address: String, murder_host: String) {
    let (connection_sender, connection_receiver) = channel();
    let (to_server_sender, to_server_receiver) = channel();
    thread::spawn(move || {
        let tcp = TcpListener::bind(websocket_address).unwrap();
        connection_handler(connection_sender, tcp, murder_host);
    });
    thread::spawn(move || {
        player_handler(connection_receiver, to_server_sender);
    });
//    thread::spawn(move || {
//        Game::new(to_server_receiver).main();
//    });
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
    to_server: Sender<GameEvent>,
) {
}

struct GameEvent;

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
    use super::*;
    use std::time::{Duration};
    #[test]
    fn it_works() {
        listen("0.0.0.0:12345".to_string(), "127.0.0.1".to_string());
        TcpStream::connect("localhost:12345").unwrap();
        thread::sleep(Duration::from_secs(3));
    }
}
