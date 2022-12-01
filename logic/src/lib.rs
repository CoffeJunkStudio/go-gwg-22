use serde::Deserialize;
use serde::Serialize;


pub mod generator;
pub mod resource;
pub mod state;
pub mod terrain;
pub mod units;


pub use nalgebra_glm as glm;
use state::Reefing;
use state::WorldState;
use terrain::Terrain;
use units::BiPolarFraction;

pub type StdRng = rand_pcg::Pcg64;



/// The size (edge length) of a terrain tile, in meter
pub const TILE_SIZE: u32 = 4;

/// The bounding-box "diameter" of a harbor, in meter
pub const HARBOR_SIZE: f32 = 3.;

/// The effect "diameter" within which a player an interact with a harbor, in meter
pub const HARBOR_EFFECT_SIZE: f32 = 6.;

/// The maximum speed of the player while trading.
pub const HARBOR_MAX_SPEED: f32 = 1.;

/// The maximum speed of the player at which a ship is docked.
pub const HARBOR_DOCKING_SPEED: f32 = 0.8;

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
