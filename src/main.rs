extern crate metrolti_lib;

pub fn main() {
    metrolti_lib::listen("localhost:3004".to_string(), String::new());
}

