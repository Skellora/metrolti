extern crate metrolti_lib;

extern crate serde;
extern crate serde_json;

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

#[derive(Debug)]
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
        DemoAction::WaitTime(2),
        DemoAction::Act(game::PlayerAction::ConnectStations(game::StationId(1), game::StationId(0))),
        DemoAction::Act(game::PlayerAction::ConnectStations(game::StationId(0), game::StationId(2))),
    ];
    let mut actions = actions_list.iter();
    let mut curr = actions.next();
    loop {
        let message = ws.read_message();
        let progress;
        match curr {
            None => { return; }
            Some(&DemoAction::WaitMessage(ref m)) => {
                progress = m == message.unwrap().to_text().unwrap();
            }
            Some(&DemoAction::WaitTime(ref s)) => {
                thread::sleep(Duration::from_secs(*s));
                progress = true;
            }
            Some(&DemoAction::Act(ref m)) => {
                let serialized = serde_json::to_string(m).expect("demo serialize");
                ws.write_message(tungstenite::Message::text(serialized)).expect("demo write");
                progress = true;
            }

        }
        if progress {
            curr = actions.next();
            println!("Moving on to {:?}", curr);
        }
    }
}

