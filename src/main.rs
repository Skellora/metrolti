extern crate metrolti_lib;

extern crate tungstenite;
extern crate url;

use metrolti_lib::server as server;
use metrolti_lib::metro_game as game;
use metrolti_lib::ticks::TPSTicker;
use metrolti_lib::web as web;

use std::thread;

use url::Url;
use std::time::{Duration};

pub fn main() {
    thread::spawn(|| web::startup_web_frontend("localhost:3005".to_string(), "localhost:3004".to_string(), "./www/static/".to_string()));
    thread::spawn(|| demo_player("ws://localhost:3004"));
    server::listen::<game::MetroGame<TPSTicker>>("localhost:3004".to_string(), String::new());
}

enum DemoAction {
    WaitMessage(String),
    WaitTime(u64),
    Act(game::PlayerAction),
}

fn demo_player(addr: &str) {
    println!("Starting demo player");
    let mut ws = {
        let uri = Url::parse(addr).unwrap();
        loop {
            if let Ok((mut ws, _resp)) = tungstenite::connect(uri.clone()) {
                break ws;
            }
        }
    };
    let actions_list = vec![
        DemoAction::WaitMessage("{\"LobbyCount\":2}".to_string()),
        DemoAction::WaitTime(2),
        DemoAction::Act(game::PlayerAction::StartGame),
    ];
    let mut actions = actions_list.iter();
    let mut curr = actions.next();
    loop {
        let message = ws.read_message();
        let mut progress = false;
        match curr {
            None => { return; }
            Some(&DemoAction::WaitMessage(ref m)) => {
                progress = m == message.unwrap().to_text().unwrap();
            }
            Some(&DemoAction::WaitTime(ref s)) => {
                thread::sleep(Duration::from_secs(*s));
                progress = true;
            }
            Some(&DemoAction::Act(_)) => {
                ws.write_message(tungstenite::Message::text("{\"StartGame\":null}".to_string())).expect("demo write");
            }

        }
        if progress {
            curr = actions.next();
        }
    }
}

