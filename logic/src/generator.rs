//! World generator sub module
//!

use std::f32::consts::TAU;

use nalgebra_glm::vec2;
use noise::Seedable;
use rand::Rng;
use strum::IntoEnumIterator;

use crate::state::Harbor;
use crate::state::WorldState;
use crate::units::Elevation;
use crate::units::TileType;
use crate::ResourcePack;
use crate::ResourcePackContent;
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
			*tt.1 = Elevation(rng.gen_range(Elevation::DEEPEST.0..Elevation::HIGHEST.0));
			//*tt.1 = Elevation(rng.gen_range((-6)..(-4)));
		}

		// One resource per tile (on average)
		let resource_amount =
			setting.edge_length as f32 * setting.edge_length as f32 * setting.resource_density;

		let resources = (0..(resource_amount as u32))
			.map(|_| ResourcePack::new(terrain.random_location(&mut rng), rng.gen(), &mut rng))
			.collect();

		// One harbour per 128 tiles (on average)
		let harbor_amount =
			(setting.edge_length as f32 * setting.edge_length as f32 / 256.).max(1.0);

		let harbors = (0..(harbor_amount as u32))
			.map(|_| {
				Harbor {
					loc: terrain.random_passable_location(&mut rng),
					orientation: rng.gen::<f32>() * TAU,
				}
			})
			.collect();

		let seed: u64 = rng.gen();

		World {
			init: WorldInit {
				terrain,
				seed,
				dbg: Default::default(),
			},
			state: WorldState {
				resources,
				harbors,
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

		// Tile generation
		let noise = noise::Perlin::new().set_seed(rng.gen());
		for (cord, tt) in terrain.iter_mut() {
			use noise::NoiseFn;

			let value = noise.get([
				cord.x as f64 * PERLIN_NOISE_FACTOR,
				cord.y as f64 * PERLIN_NOISE_FACTOR,
			]);

			*tt = Elevation(((value - 0.8) * 10.) as i16);
		}

		let map_area =
			setting.edge_length as f32 * setting.edge_length as f32 * setting.resource_density;


		// Harbor spawning

		// One harbour per 256 tiles (on average)
		let harbor_amount =
			(setting.edge_length as f32 * setting.edge_length as f32 / 256.).max(1.0) as usize;

		let mut harbors = Vec::new();
		// Add all the harbors
		while harbors.len() < harbor_amount {
			let loc = terrain.random_passable_location(&mut rng);
			let elev = *terrain.get(loc.try_into().unwrap());

			// Ensure a harbor only spawn within shallow water
			if !(TileType::ShallowWater.lowest() <= elev
				&& elev <= TileType::ShallowWater.highest())
			{
				continue;
			}

			let harbor = Harbor {
				loc,
				orientation: rng.gen::<f32>() * TAU,
			};
			harbors.push(harbor);
		}


		// Resource spawning

		let mut resources = Vec::new();
		for cnt in ResourcePackContent::iter() {
			// One resource per tile (on average)
			let resource_amount = map_area * cnt.spawn_density;
			let school_size = rng.gen_range(cnt.schooling_size.clone());

			let mut current_set = Vec::new();

			while current_set.len() < resource_amount as usize {
				let loc = terrain.random_passable_location(&mut rng);
				let loc_elev = terrain.get(loc.try_into().unwrap());

				if !cnt.spawn_location.contains(loc_elev) {
					continue;
				}

				let org = ResourcePack::new(loc, cnt, &mut rng);

				if org.elevation < *loc_elev {
					continue;
				}

				current_set.extend((0..school_size).map(|_| {
					let mut clone = org.clone();
					clone.phase += rng.gen_range(0.0..TAU) / 20.;
					clone.origin.0 += vec2(rng.gen(), rng.gen()) * 1.;
					clone
				}))
			}

			resources.extend(current_set);
		}

		let seed: u64 = rng.gen();

		World {
			init: WorldInit {
				terrain,
				seed,
				dbg: Default::default(),
			},
			state: WorldState {
				resources,
				harbors,
				..Default::default()
			},
		}
	}
}
