
#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub struct PlayerId(u16);

impl PlayerId {
    pub fn new(id: u16) -> Self {
        PlayerId(id)
    }
}

