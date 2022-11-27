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
pub const VEHICLE_SIZE: f32 = 1.3;

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
}

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
}

const NO_SCHOOLING: Range<usize> = 1..2;

enumeraties::props! {
	impl Deref for ResourcePackContent as const ResourcePackStats {
		Self::Fish0 => {
			weight: 10,
			value: 12,
			schooling_size: 4..10,
			spawn_density: 1.0,
			spawn_elevation: Elevation(-18)..Elevation(-12),
			spawn_location: Elevation(-18)..Elevation(-12),
		}
		Self::Fish1 => {
			weight: 20,
			value: 25,
			schooling_size: NO_SCHOOLING,
			spawn_density: 0.1,
			spawn_elevation: Elevation(-5)..Elevation(0),
			spawn_location: Elevation(-12)..Elevation(0),
		}
		_ => {
			weight: 10,
			value: 12,
			schooling_size: 3..4,
			spawn_density: 0.0,
			spawn_elevation: Elevation(-18)..Elevation(-7),
			spawn_location: Elevation(-18)..Elevation(-7),
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
			params: (rng.gen_range(-9..=-1), rng.gen_range(2..=10)), // (0,0) for starfish
			phase: rng.gen_range(0.0..TAU),
			speed_factor: 10, // 1 for starfish
			backwards: rng.gen(),
		}
	}

	pub fn update(&mut self, current_tick: Tick) {
		// Forwardness factor, `1` if forward, `-1` if backwards
		let forwardness = (1 - 2 * self.backwards as i8) as f32;

		// The total animation cycle duration
		let duration = u32::from(1 + self.params.0.unsigned_abs() + self.params.1.unsigned_abs())
			* 10 / self.speed_factor;
		let duration = duration * (FISH_ANIM_BASE_DURATION * u32::from(TICKS_PER_SECOND));
		// The current progress through the animation
		let progress = forwardness
			* (self.phase + TAU * (current_tick.0 % u64::from(duration)) as f32 / duration as f32);

		// The position function
		let base = vec2(progress.sin(), progress.cos());
		let first = vec2(
			(progress * self.params.0 as f32).sin(),
			(progress * self.params.0 as f32).cos(),
		);
		let second = vec2(
			(progress * self.params.1 as f32).sin(),
			(progress * self.params.1 as f32).cos(),
		);
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
