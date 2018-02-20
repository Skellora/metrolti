use std::net::TcpListener;
use std::sync::mpsc::channel;
use std::thread;

use game::Game;
use player::*;

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


#[cfg(test)]
mod tests {
    extern crate url;

    use super::*;
    use events::*;
    use std::net::TcpStream;
    use std::sync::mpsc::Receiver;
    use std::time::{Duration};
    use self::url::Url;
    use tungstenite;

    struct EchoGame {
        r: Receiver<InputEvent>,
    }

    impl Game for EchoGame {
        fn new(r: Receiver<InputEvent>) -> Self {
            EchoGame {
                r: r
            }
        }

        fn main(&mut self) {
            let mut p = None;
            loop {
                match self.r.recv().unwrap() {
                    InputEvent::Connection(_, player) => { p = Some(player); },
                    InputEvent::Message(_, _) => { p.take().map(|player| player.send_message(StateUpdate)); },
                    _ => {},
                }
            }
        }
    }

    #[test]
    fn server_comms1() {
        listen::<EchoGame>("0.0.0.0:12345".to_string(), "127.0.0.1".to_string());

        let uri = Url::parse("ws://localhost:12345").unwrap();
        let mut ws = tungstenite::connect(uri).unwrap().0;
        ws.write_message(tungstenite::Message::text("null".to_string()));

        assert_eq!("null", ws.read_message().unwrap().to_text().unwrap());


        TcpStream::connect("localhost:12345").unwrap();
        thread::sleep(Duration::from_secs(1));

        assert!(TcpStream::connect("localhost:12345").is_err());
    }
}
