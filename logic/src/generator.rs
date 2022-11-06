//! World generator sub module
//!

use noise::Seedable;
use rand::Rng;

use crate::state::WorldState;
use crate::units::Elevation;
use crate::ResourcePack;
use crate::Terrain;
use crate::World;
use crate::WorldInit;


const PERLIN_NOISE_FACTOR: f64 = 1. / core::f64::consts::PI / 2.;


/// The basic map output settings
pub struct Setting {
	/// Amount of tiles along each axis in tiles
	pub edge_length: u16,

	/// Resource density
	pub resource_density: f32,
}

/// A world generator
pub trait Generator {
	fn generate<R: Rng>(&self, setting: &Setting, rng: R) -> World;
}

/// Fully random, no structure
pub struct WhiteNoise;

impl Generator for WhiteNoise {
	fn generate<R: Rng>(&self, setting: &Setting, mut rng: R) -> World {
		let mut terrain = Terrain::new(setting.edge_length);

		for tt in terrain.iter_mut() {
			*tt.1 = Elevation(rng.gen_range(-10..10));
		}

		// One resource per tile (on average)
		let resource_amount =
			setting.edge_length as f32 * setting.edge_length as f32 * setting.resource_density;

		let resources = (0..(resource_amount as u32))
			.map(|_| {
				ResourcePack {
					content: rng.gen(),
					loc: terrain.random_location(&mut rng),
				}
			})
			.collect();

		let seed: u64 = rng.gen();

		World {
			init: WorldInit {
				terrain,
				seed,
			},
			state: WorldState {
				resources,
				..Default::default()
			},
		}
	}
}



/// Smooth Perlin noise
pub struct PerlinNoise;

impl Generator for PerlinNoise {
	fn generate<R: Rng>(&self, setting: &Setting, mut rng: R) -> World {
		let mut terrain = Terrain::new(setting.edge_length);

		let noise = noise::Perlin::new().set_seed(rng.gen());
		for (cord, tt) in terrain.iter_mut() {
			use noise::NoiseFn;

			let value = noise.get([
				cord.x as f64 * PERLIN_NOISE_FACTOR,
				cord.y as f64 * PERLIN_NOISE_FACTOR,
			]);

			*tt = Elevation(((value - 0.8) * 10.) as i16);
		}

		// One resource per tile (on average)
		let resource_amount =
			setting.edge_length as f32 * setting.edge_length as f32 * setting.resource_density;

		let resources = (0..(resource_amount as u32))
			.map(|_| {
				ResourcePack {
					content: rng.gen(),
					loc: terrain.random_passable_location(&mut rng),
				}
			})
			.collect();

		let seed: u64 = rng.gen();

		World {
			init: WorldInit {
				terrain,
				seed,
			},
			state: WorldState {
				resources,
				..Default::default()
			},
		}
	}
}
