use enum_map::Enum;
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
use units::Fish;
use units::Location;

pub type StdRng = rand_pcg::Pcg64;



/// The size (edge length) of a terrain tile, in meter
pub const TILE_SIZE: u32 = 4;

/// The "diameter" of the player's car.
pub const VEHICLE_SIZE: f32 = 1.3;

/// The mass of a empty vehicle, in kilogram
const VEHICLE_DEADWEIGHT: f32 = 100.0;

/// The physical size ("diameter") of a water resource pack.
pub const RESOURCE_PACK_FISH_SIZE: f32 = 0.8;

/// The amount of fuel in each fuel resource pack
pub const RESOURCE_PACK_FISH_AMOUNT: Fish = Fish(1.);

/// The diameter of the tier, in meter
const TIRE_DIAMETER: f32 = 0.4;
/// Gives the speed in m/s per axle rpm
const TIRE_SPEED_PER_RPM: f32 = core::f32::consts::PI * TIRE_DIAMETER / 60.0;

/// Gives the engine rpm per axle rpm
const GEAR_BASE_RATION: f32 = 0.1;
/// Gives ration of the engine rpm per axle rpm for each gear
const GEAR_RATIO_PROGRESSION: f32 = core::f32::consts::SQRT_2;

/// Gives the lower allowed bound for the engine rpm, if the calculated
/// engine rpm falls below this threshold, the engine stalls.
pub const ENGINE_STALL_RPM: f32 = 950.0;

/// The engine power at full throttle and ideal RPM, in watt
pub const ENGINE_POWER: f32 = 5_000.0;

/// The optimal engine rpm, i.e. yielding ful power, in rpm
pub const ENGINE_IDEAL_RPM: f32 = 3_000.0;

/// The work produced by one kilogram of fuel, in J/kg
pub const ENGINE_WORK_PER_FUEL: f32 = 25_000.0;

/// Scalar factor influencing the strength of ground based friction.
///
/// This kind of friction gets stronger if the vehicle moves faster over ground.
pub const FRICTION_GROUND_SPEED_FACTOR: f32 = 0.02;

/// Scalar factor influencing the strength of gronud based friction when sliding
pub const FRICTION_CROSS_SPEED_FACTOR: f32 = 0.05;

/// Scalar factor influencing the strength of motor friction.
///
/// This kind of friction gets stronger if the motor runs at higher RPM.
pub const FRICTION_MOTOR_FACTOR: f32 = 0.0001;

/// The maximum steering angle in radians per steering.
pub const VEHICLE_MAX_STEERING_ANGLE: f32 = core::f32::consts::FRAC_PI_3; // = 60 deg

/// The inner length of the vehicle, it this the distance between the front and back wheels in meter
pub const VEHICLE_WHEEL_BASE: f32 = 0.9 * VEHICLE_SIZE;

/// The maximum breaking power in m/s²
pub const BREAKING_DEACCL: f32 = 3.0;

/// The players continues water consumption in kg/s
pub const WATER_CONSUMPTION: f32 = 0.01;

/// Maximum amount of traction
pub const MAX_TRACTION: f32 = 1.0;



/// Gives the resource type that can be in a resource pack
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[derive(Enum)]
#[derive(strum::EnumIter)]
#[derive(standard_dist::StandardDist)]
pub enum ResourcePackContent {
	Fish,
}

/// A collectable resource on the ground
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct ResourcePack {
	/// The type of the resource
	pub content: ResourcePackContent,
	/// The location of the resource in meter
	pub loc: Location,
}


/// The entire game world
#[derive(Debug, Clone)]
pub struct World {
	pub init: WorldInit,
	pub state: WorldState,
}

/// The static initial part of the world
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct WorldInit {
	pub terrain: Terrain,
}



/// Represents the input state of a player
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub struct Input {
	/// The active gear
	pub reefing: Reefing,
	/// The current steering as fraction from -1.0 to +1.0.
	///
	/// The meaning is as follows:
	/// * `-1.0` means full deflection towards the left
	/// * `0.0` means neutral, straight ahead
	/// * `+1.0` means full deflection towards the right
	pub rudder: BiPolarFraction,
}
