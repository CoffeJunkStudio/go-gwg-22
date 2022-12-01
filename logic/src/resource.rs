use std::f32::consts::TAU;
use std::ops::Range;

use enum_map::Enum;
use glm::vec2;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

use super::glm;
use crate::units::Elevation;
use crate::units::Location;
use crate::units::Tick;
use crate::FISH_ANIM_BASE_DURATION;
use crate::TICKS_PER_SECOND;



/// Gives the resource type that can be in a resource pack
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[derive(Enum)]
#[derive(strum::EnumIter)]
#[derive(standard_dist::StandardDist)]
pub enum ResourcePackContent {
	Fish0,
	Fish1,
	Fish2,
	Fish3,
	Fish4,
	Fish5,
	Fish6,
	Fish7,
	Shoe0,
	Shoe1,
	Starfish0,
	Starfish1,
	Starfish2,
	Starfish3,
	Starfish4,
	Grass0,
	Grass1,
}

#[derive(Debug, Clone)]
pub struct ResourcePackStats {
	/// The resource weight in kg
	pub weight: u32,
	/// The value of the resource in money
	pub value: u64,
	/// The number of fishies to spawn together
	pub schooling_size: Range<usize>,
	/// The spawn frequency described as density in resources per tile
	pub spawn_density: f32,
	/// Specifies at which depths the resource appears
	pub spawn_elevation: Range<Elevation>,
	/// Specifies in which waters it resource may spawn
	pub spawn_location: Range<Elevation>,
	/// The ranges for the parameters of the animation curve
	pub params_range: (Range<i8>, Range<i8>),
	/// The range of speed factor
	pub speed_factor: Range<u32>,
}

const NO_SCHOOLING: Range<usize> = 1..2;

enumeraties::props! {
	impl Deref for ResourcePackContent as const ResourcePackStats {
		Self::Fish0 => {
			weight: 10,
			value: 12,
			schooling_size: 4..10,
			spawn_density: 0.35,
			spawn_elevation: Elevation(-18)..Elevation(-12),
			spawn_location: Elevation(-18)..Elevation(-12),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish1 => {
			weight: 20,
			value: 25,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.05,
			spawn_elevation: Elevation(-5)..Elevation(0),
			spawn_location: Elevation(-12)..Elevation(0),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish2 => {
			weight: 15,
			value: 17,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.3,
			spawn_elevation: Elevation(-12)..Elevation(-5),
			spawn_location: Elevation(-18)..Elevation(-5),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish3 => {
			weight: 8,
			value: 8,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.1,
			spawn_elevation: Elevation(-12)..Elevation(-5),
			spawn_location: Elevation(-12)..Elevation(0),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish4 => {
			weight: 5,
			value: 10,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.06,
			spawn_elevation: Elevation(-5)..Elevation(0),
			spawn_location: Elevation(-5)..Elevation(0),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish5 => {
			weight: 6,
			value: 5,
			schooling_size: 10..15,
			spawn_density: 0.5,
			spawn_elevation: Elevation(-18)..Elevation(0),
			spawn_location: Elevation(-18)..Elevation(0),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish6 => {
			weight: 7,
			value: 6,
			schooling_size: 5..7,
			spawn_density: 0.5,
			spawn_elevation: Elevation(-18)..Elevation(0),
			spawn_location: Elevation(-18)..Elevation(-5),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Fish7 => {
			weight: 18,
			value: 19,
			schooling_size: 1..3,
			spawn_density: 0.1,
			spawn_elevation: Elevation(-12)..Elevation(-5),
			spawn_location: Elevation(-12)..Elevation(-5),
			params_range: (-9..0, 2..11),
			speed_factor: 90..110,
		}
		Self::Starfish0 => {
			weight: 3,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.05,
			spawn_elevation: Elevation(-3)..Elevation(0),
			spawn_location: Elevation(-4)..Elevation(0),
			params_range: (0..1,0..1),
			speed_factor: 20..30,
		}
		Self::Starfish1 => {
			weight: 5,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.04,
			spawn_elevation: Elevation(-1)..Elevation(0),
			spawn_location: Elevation(-12)..Elevation(0),
			params_range: (0..1,0..1),
			speed_factor: 20..30,
		}
		Self::Starfish2 => {
			weight: 4,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.04,
			spawn_elevation: Elevation(-5)..Elevation(0),
			spawn_location: Elevation(-12)..Elevation(-5),
			params_range: (0..1,0..1),
			speed_factor: 20..30,
		}
		Self::Starfish3 => {
			weight: 3,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.02,
			spawn_elevation: Elevation(-18)..Elevation(-12),
			spawn_location: Elevation(-18)..Elevation(-12),
			params_range: (0..1,0..1),
			speed_factor: 20..30,
		}
		Self::Starfish4 => {
			weight: 3,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.02,
			spawn_elevation: Elevation(-12)..Elevation(-5),
			spawn_location: Elevation(-12)..Elevation(0),
			params_range: (0..1,0..1),
			speed_factor: 20..30,
		}
		Self::Grass0 => {
			weight: 9,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 1.0,
			spawn_elevation: Elevation(-1)..Elevation(0),
			spawn_location: Elevation(-4)..Elevation(0),
			params_range: (0..1,0..1),
			speed_factor: 1..10,
		}
		Self::Grass1 => {
			weight: 10,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 1.0,
			spawn_elevation: Elevation(-1)..Elevation(0),
			spawn_location: Elevation(-6)..Elevation(-3),
			params_range: (0..1,0..1),
			speed_factor: 5..15,
		}
		Self::Shoe0 => {
			weight: 5,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.03,
			spawn_elevation: Elevation(-1)..Elevation(0),
			spawn_location: Elevation(-12)..Elevation(0),
			params_range: (0..1,0..1),
			speed_factor: 1..15,
		}
		Self::Shoe1 => {
			weight: 5,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.03,
			spawn_elevation: Elevation(-1)..Elevation(0),
			spawn_location: Elevation(-18)..Elevation(-5),
			params_range: (0..1,0..1),
			speed_factor: 1..20,
		}
	}
}


/// A collectable resource on the ground
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct ResourcePack {
	/// The type of the resource
	pub content: ResourcePackContent,
	/// The location of the resource in meter
	pub loc: Location,
	/// The orientation of the resource, zero is world x axis
	pub ori: f32,
	/// The depth of the fish
	pub elevation: Elevation,

	/// The origin location of the resource in meter
	pub origin: Location,
	/// Animation parameters
	pub params: (i8, i8),
	/// Animation phase offset
	pub phase: f32,
	/// Animation speed
	pub speed_factor: u32,
	/// Whether to play the animation backwards
	pub backwards: bool,
}
impl ResourcePack {
	pub fn new<R: Rng>(loc: Location, kind: ResourcePackContent, mut rng: R) -> Self {
		Self {
			content: kind,
			loc: Default::default(),
			elevation: rng.gen_range(kind.spawn_elevation.clone()),
			ori: 0.,
			origin: loc,
			params: (
				rng.gen_range(kind.params_range.0.clone()),
				rng.gen_range(kind.params_range.1.clone()),
			), // (0,0) for starfish
			phase: rng.gen_range(0.0..TAU),
			speed_factor: rng.gen_range(kind.speed_factor.clone()),
			backwards: rng.gen(),
		}
	}

	pub fn update(&mut self, current_tick: Tick) {
		// Forwardness factor, `1` if forward, `-1` if backwards
		let forwardness = (1 - 2 * self.backwards as i8) as f32;

		// The total animation cycle duration
		let duration = u32::from(1 + self.params.0.unsigned_abs() + self.params.1.unsigned_abs())
			* 100 / self.speed_factor;
		let duration = duration * (FISH_ANIM_BASE_DURATION * u32::from(TICKS_PER_SECOND));
		// The current progress through the animation
		let progress = forwardness
			* (self.phase + TAU * (current_tick.0 % u64::from(duration)) as f32 / duration as f32);

		// The position function
		let base = vec2(progress.sin(), progress.cos());
		let first = if self.params.0 == 0 {
			vec2(0., 0.)
		} else {
			vec2(
				(progress * self.params.0 as f32).sin(),
				(progress * self.params.0 as f32).cos(),
			)
		};
		let second = if self.params.0 == 0 {
			vec2(0., 0.)
		} else {
			vec2(
				(progress * self.params.1 as f32).sin(),
				(progress * self.params.1 as f32).cos(),
			)
		};
		self.loc = Location(self.origin.0 + base + first + second);

		// Derivation of the position function (i.e. the orientation vector)
		let d_base = vec2(progress.cos(), -progress.sin());
		let d_first = vec2(
			(progress * self.params.0 as f32).cos() * self.params.0 as f32,
			-(progress * self.params.0 as f32).sin() * self.params.0 as f32,
		);
		let d_second = vec2(
			(progress * self.params.1 as f32).cos() * self.params.1 as f32,
			-(progress * self.params.1 as f32).sin() * self.params.1 as f32,
		);
		let d_vec = forwardness * (d_base + d_first + d_second);

		self.ori = f32::atan2(d_vec.y, d_vec.x);
	}
}
