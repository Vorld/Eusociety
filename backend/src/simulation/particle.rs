use serde::{Serialize, Deserialize};
use std::cell::RefCell;
use rand::rngs::ThreadRng;
use rand::Rng;
use crate::simulation::entity::Entity;

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ParticleState {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Particle {
    pub id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub acceleration_x: f32,
    pub acceleration_y: f32,
}

thread_local! {
    static RNG: RefCell<ThreadRng> = RefCell::new(rand::thread_rng());
}

const DAMPING: f32 = 0.98;
const MAX_SPEED: f32 = 1000.0;
const RANDOM_FORCE: f32 = 500.0;
const BOUNDARY_X: f32 = 6000.0;
const BOUNDARY_Y: f32 = 6000.0;

impl Particle {
    pub fn new(id: usize, pos_x: f64, pos_y: f64) -> Self {
        Self {
            id: id as u32,
            pos_x: pos_x as f32,
            pos_y: pos_y as f32,
            velocity_x: 0.0,
            velocity_y: 0.0,
            acceleration_x: 0.0,
            acceleration_y: 0.0,
        }
    }

    #[inline(always)]
    fn apply_boundary_conditions(&mut self) {
        if self.pos_x < 0.0 {
            self.pos_x = 0.0;
            self.velocity_x = -self.velocity_x * DAMPING;
        } else if self.pos_x > BOUNDARY_X {
            self.pos_x = BOUNDARY_X;
            self.velocity_x = -self.velocity_x * DAMPING;
        }

        if self.pos_y < 0.0 {
            self.pos_y = 0.0;
            self.velocity_y = -self.velocity_y * DAMPING;
        } else if self.pos_y > BOUNDARY_Y {
            self.pos_y = BOUNDARY_Y;
            self.velocity_y = -self.velocity_y * DAMPING;
        }
    }
}

impl Entity for Particle {
    fn update(&mut self, dt: f32) {
        // Apply random force occasionally
        if rand::random::<f32>() < 0.1 {
            RNG.with(|rng| {
                let mut rng = rng.borrow_mut();
                self.acceleration_x = rng.gen_range(-RANDOM_FORCE..RANDOM_FORCE);
                self.acceleration_y = rng.gen_range(-RANDOM_FORCE..RANDOM_FORCE);
            });
        }

        // Verlet integration
        self.velocity_x += self.acceleration_x * dt;
        self.velocity_y += self.acceleration_y * dt;
        
        // Apply damping
        self.velocity_x *= DAMPING;
        self.velocity_y *= DAMPING;
        
        // Clamp velocity
        self.velocity_x = self.velocity_x.clamp(-MAX_SPEED, MAX_SPEED);
        self.velocity_y = self.velocity_y.clamp(-MAX_SPEED, MAX_SPEED);

        // Update position
        self.pos_x += self.velocity_x * dt;
        self.pos_y += self.velocity_y * dt;

        // Handle boundaries
        self.apply_boundary_conditions();
    }

    fn serialize(&self) -> Vec<u8> {
        let state = ParticleState {
            id: self.id,
            x: self.pos_x,
            y: self.pos_y,
        };
        bincode::serialize(&state).unwrap_or_default()
    }
}