use std::net::TcpListener;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use game::Game;
use player::*;
use ticks::*;

pub fn listen<G: Game<TPSTicker>>(websocket_address: String, murder_host: String) {
    let (connection_sender, connection_receiver) = channel();
    let (to_server_sender, to_server_receiver) = channel();
    thread::spawn(move || {
        let tcp = TcpListener::bind(websocket_address).unwrap();
        connection_handler(connection_sender, tcp, murder_host);
        println!("Closing connection handler");
    });
    thread::spawn(move || {
        player_handler(connection_receiver, to_server_sender);
        println!("Closing player handler");
    });
    let tick_rate = Duration::from_millis(1000/30);
    let ticker = TPSTicker::new(tick_rate);
    G::new(to_server_receiver, ticker).main();
    println!("Game exiting");
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

    impl<T: Ticker> Game<T> for EchoGame {
        fn new(r: Receiver<InputEvent>, _: T) -> Self {
            EchoGame {
                r: r
            }
        }

        fn main(&mut self) {
            let mut p = None;
            loop {
                match self.r.recv().unwrap() {
                    InputEvent::Connection(_, player) => { p = Some(player); },
                    InputEvent::PlayerAction(_, _) => { p.take().map(|player| player.send_message(StateUpdate::LobbyCount(1))); },
                    _ => {},
                }
            }
        }
    }

    #[test]
    fn server_comms1() {

        thread::spawn(||
            listen::<EchoGame>("127.0.0.1:12345".to_string(), "127.0.0.1".to_string())
        );

        let mut ws = {
            let uri = Url::parse("ws://127.0.0.1:12345").unwrap();
            loop {
                if let Ok((mut ws, _resp)) = tungstenite::connect(uri.clone()) {
                    break ws;
                }
            }
        };
        assert!(ws.write_message(tungstenite::Message::text("{\"StartGame\":null}".to_string())).is_ok());

        assert_eq!("{\"LobbyCount\":1}", ws.read_message().unwrap().to_text().unwrap());

        TcpStream::connect("127.0.0.1:12345").unwrap();
        thread::sleep(Duration::from_millis(100));

        assert!(TcpStream::connect("127.0.0.1:12345").is_err());
    }
}
