/// The Entity trait defines the interface for simulation entities.
pub trait Entity: Send + Sync {
    fn update(&mut self, dt: f32);
    fn serialize(&self) -> Vec<u8>;
}
