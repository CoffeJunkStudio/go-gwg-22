use std::f32::consts::TAU;
use std::ops::Range;

use enum_map::Enum;
use glm::vec2;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

pub mod generator;
pub mod state;
pub mod terrain;
pub mod units;

pub use nalgebra_glm as glm;
use state::Reefing;
use state::WorldState;
use terrain::Terrain;
use units::BiPolarFraction;
use units::Elevation;
use units::Fish;
use units::Location;
use units::Tick;

pub type StdRng = rand_pcg::Pcg64;

/// The size (edge length) of a terrain tile, in meter
pub const TILE_SIZE: u32 = 4;

/// The bounding-box "diameter" of a harbor, in meter
pub const HARBOR_SIZE: f32 = 3.;

/// The effect "diameter" within which a player an interact with a harbor, in meter
pub const HARBOR_EFFECT_SIZE: f32 = 6.;

/// The maximum speed of the player while trading.
pub const HARBOR_MAX_SPEED: f32 = 1.;

/// The "diameter" of the player's car.
pub const VEHICLE_SIZE: f32 = 2.6;

/// The mass of a empty vehicle, in kilogram
const VEHICLE_DEADWEIGHT: f32 = 100.0;

/// The physical size ("diameter") of a water resource pack.
pub const RESOURCE_PACK_FISH_SIZE: f32 = 0.8;

/// Scalar factor influencing the strength of ground based friction.
///
/// This kind of friction gets stronger if the vehicle moves faster over ground.
pub const FRICTION_GROUND_SPEED_FACTOR: f32 = 0.1;

/// Scalar factor influencing the strength of gronud based friction when sliding
pub const FRICTION_CROSS_SPEED_FACTOR: f32 = 0.8;

/// The maximum steering angle in radians per steering.
pub const VEHICLE_MAX_STEERING_ANGLE: f32 = core::f32::consts::FRAC_PI_3; // = 60 deg

/// The inner length of the vehicle, it this the distance between the front and back wheels in meter
pub const VEHICLE_WHEEL_BASE: f32 = 0.9 * VEHICLE_SIZE;

/// Maximum amount of traction
pub const MAX_TRACTION: f32 = 0.5;

/// The interval between wind changes in seconds
pub const WIND_CHANGE_INTERVAL: u16 = 10;

/// The maximum wind speed in m/s
pub const MAX_WIND_SPEED: f32 = 15.0;

/// Number of fish variants
pub const FISH_TYPES: u8 = 8;

/// The base duration of the fish animation in seconds
pub const FISH_ANIM_BASE_DURATION: u32 = 3;

/// Target logical ticks per second
pub const TICKS_PER_SECOND: u16 = 60;

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
	weight: u32,
	/// The value of the resource in money
	value: u64,
	/// The number of fishies to spawn together
	schooling_size: Range<usize>,
	/// The spawn frequency described as density in resources per tile
	spawn_density: f32,
	/// Specifies at which depths the resource appears
	spawn_elevation: Range<Elevation>,
	/// Specifies in which waters it resource may spawn
	spawn_location: Range<Elevation>,
	///
	params_range: (Range<i8>, Range<i8>),
	///
	speed_factor: Range<u32>,
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
			spawn_density: 0.5,
			spawn_elevation: Elevation(-1)..Elevation(0),
			spawn_location: Elevation(-4)..Elevation(0),
			params_range: (0..1,0..1),
			speed_factor: 1..10,
		}
		Self::Grass1 => {
			weight: 10,
			value: 1,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.5,
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

#[derive(Debug, Clone, Copy, Default)]
#[derive(Serialize, Deserialize)]
pub struct DebuggingConf {
	/// Give the ship an engine which will propel the ship at a constant speed
	/// regardless of the wind.
	pub ship_engine: bool,

	/// Make the wind constantly turn
	pub wind_turning: bool,

	/// Fix the wind direction in a specific direction, in radians
	pub fixed_wind_direction: Option<f32>,
}

/// The entire game world
#[derive(Debug, Clone)]
pub struct World {
	pub init: WorldInit,
	pub state: WorldState,
}
impl World {
	// nothing, yet
}

/// The static initial part of the world
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct WorldInit {
	/// Defines the map tiles
	pub terrain: Terrain,
	/// Random seed used for this game
	pub seed: u64,
	/// Debugging configuration
	pub dbg: DebuggingConf,
}

/// Represents the input state of a player
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub struct Input {
	/// The wanted reefing setting
	pub reefing: Reefing,

	/// The current steering as fraction from -1.0 to +1.0.
	///
	/// The meaning is as follows:
	/// * `-1.0` means full deflection towards the left
	/// * `0.0` means neutral, straight ahead
	/// * `+1.0` means full deflection towards the right
	pub rudder: BiPolarFraction,
}
