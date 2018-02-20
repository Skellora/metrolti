use std::fmt::Debug;

pub trait SoftExpect<E> {
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

