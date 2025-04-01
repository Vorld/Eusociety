use crate::World;

pub trait System {
    fn run(&mut self, world: &mut World);
}
