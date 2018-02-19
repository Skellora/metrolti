extern crate metrolti_lib;

pub fn main() {
    metrolti_lib::listen::<metrolti_lib::MetroGame>("localhost:3004".to_string(), String::new());
}

