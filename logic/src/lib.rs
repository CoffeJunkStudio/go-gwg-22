use enum_map::Enum;
use nalgebra_glm::Vec2;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;


pub mod generator;
pub mod state;
pub mod units;

pub use nalgebra_glm as glm;
use state::Trim;
use state::WorldState;
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


/// The type of a terrain tile
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[derive(Enum)]
#[derive(strum::EnumIter)]
#[derive(standard_dist::StandardDist)]
pub enum TerrainType {
	/// Traversable terrain, deep water
	Deep,
	/// Traversable terrain, shallow but still passable water
	Shallow,
	/// Non-traversable terrain, where a players will beach
	Land,
}
// TODO: use enumeratis
impl TerrainType {
	pub fn is_passable(self) -> bool {
		match self {
			Self::Deep => true,
			Self::Shallow => true,
			Self::Land => false,
		}
	}
}
impl Default for TerrainType {
	fn default() -> Self {
		Self::Shallow
	}
}

/// The coordinates of a tile of the map, given by its tile axial indices
///
/// Notice that tiles are bigger than one meters, thus these coordinates are different from a location in meters.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub struct TileCoord {
	/// The tile index along the x-axis, zero-indexed
	pub x: u16,
	/// The tile index along the y-axis, zero-indexed
	pub y: u16,
}
impl TileCoord {
	/// A new coordinate for the given indices
	pub fn new(x: u16, y: u16) -> TileCoord {
		TileCoord {
			x,
			y,
		}
	}

	pub fn with_x(self, x: u16) -> Self {
		Self {
			x,
			..self
		}
	}

	pub fn with_y(self, y: u16) -> Self {
		Self {
			y,
			..self
		}
	}

	pub fn flat_map(
		self,
		convert: impl Fn(u16, u16) -> (Option<u16>, Option<u16>),
	) -> Option<Self> {
		if let (Some(x), Some(y)) = convert(self.x, self.y) {
			Some(Self {
				x,
				y,
			})
		} else {
			None
		}
	}

	/// Creates a iterator over all tiles within a map square of given edge length in tiles.
	pub fn coords(edge_length: u16) -> impl Iterator<Item = TileCoord> {
		(0..edge_length).flat_map(move |y| (0..edge_length).map(move |x| TileCoord::new(x, y)))
	}

	/// Calculate the center point location of this tile in meter
	pub fn to_location(self) -> Location {
		self.into()
	}
}
impl From<(u16, u16)> for TileCoord {
	fn from((x, y): (u16, u16)) -> Self {
		TileCoord {
			x,
			y,
		}
	}
}
impl From<TileCoord> for (u16, u16) {
	fn from(tc: TileCoord) -> Self {
		(tc.x, tc.y)
	}
}
/// Gives the coordinates of the tile below the given location
///
/// Notice, if the location is out-of-bounds of the map, so will the tile coord.
impl From<Location> for TileCoord {
	fn from(loc: Location) -> Self {
		// TODO: consider handling these errors, as well as
		// `n / TILE_SIZE > u16::MAX`, more graceful
		assert!(loc.0.x >= 0.0, "x is negative (or nan)");
		assert!(loc.0.y >= 0.0, "y is negative (or nan)");
		Self {
			x: (loc.0.x as u32 / TILE_SIZE)
				.try_into()
				.unwrap_or_else(|_| panic!("Location way too huge: {:?}", loc)),
			y: (loc.0.y as u32 / TILE_SIZE)
				.try_into()
				.unwrap_or_else(|_| panic!("Location way too huge: {:?}", loc)),
		}
	}
}
/// Gives the center point of the tile
impl From<TileCoord> for Location {
	fn from(tc: TileCoord) -> Self {
		Self(Vec2::new(
			(tc.x as u32 * TILE_SIZE) as f32 + 0.5,
			(tc.y as u32 * TILE_SIZE) as f32 + 0.5,
		))
	}
}

/// Gives the tile coordinate of the given global index.
fn coord(edge_len: u16, index: usize) -> TileCoord {
	let x = index % usize::from(edge_len);
	let y = index / usize::from(edge_len);

	let x = x.try_into().unwrap();
	let y = y.try_into().unwrap();

	TileCoord::new(x, y)
}

/// The terrain of the world.
///
/// The terrain is a square with `edge_length` tiles along each axis.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Terrain {
	/// Amount of tiles along each world axis.
	///
	/// Must not be zero.
	///
	/// Notice that this counts tiles not meters!
	pub edge_length: u16,

	/// The definition of the terrain.
	///
	/// This `Vec` has exactly `edge_length * edge_length` elements.
	/// Only use this to iterate over this if you need just the terrain types.
	/// Prefer using [get](Self::get) and [get_mut](Self::get_mut)
	pub playground: Vec<TerrainType>,
}
impl Terrain {
	/// Creates a new "flat" terrain with given edge length in tiles
	pub fn new(edge_length: u16) -> Self {
		let size = usize::from(edge_length) * usize::from(edge_length);
		let playground = vec![Default::default(); size];

		Self {
			edge_length,
			playground,
		}
	}

	/// Checks whether the given location is within the map boundary
	pub fn contains(&self, loc: Location) -> bool {
		0. <= loc.0.x && loc.0.x < self.edge_length as f32* TILE_SIZE as f32 && // nl
		0. <= loc.0.y && loc.0.y < self.edge_length as f32 * TILE_SIZE as f32
	}

	/// Calculate global tile index from tile coordinate
	fn index(&self, tc: TileCoord) -> usize {
		usize::from(tc.y) * usize::from(self.edge_length) + usize::from(tc.x)
	}

	/// Calculate tile coordinate for global tile index
	fn coord(&self, index: usize) -> TileCoord {
		coord(self.edge_length, index)
	}

	pub fn try_get(&self, tc: TileCoord) -> Option<&TerrainType> {
		self.playground.get(self.index(tc))
	}

	/// Gets tile type at given coordinate
	pub fn get(&self, tc: TileCoord) -> &TerrainType {
		let idx = self.index(tc);
		&self.playground[idx]
	}

	/// Gets mutably the tile type at given coordinate
	pub fn get_mut(&mut self, tc: TileCoord) -> &mut TerrainType {
		let idx = self.index(tc);
		&mut self.playground[idx]
	}

	/// Creates a terrain from an array of rows.
	///
	/// I.e. a tile at (x,y) would be represented by `array[x][y]`
	pub fn from_array<const N: usize>(array: [[TerrainType; N]; N]) -> Self {
		assert!(N > 0);
		let edge_length: u16 = N.try_into().unwrap();

		let mut vec = Vec::with_capacity(N * N);
		for sub in array {
			for e in sub {
				vec.push(e);
			}
		}

		Self {
			edge_length,
			playground: vec,
		}
	}

	/// Returns all valid tile coordinates.
	pub fn coords(&self) -> impl Iterator<Item = TileCoord> {
		TileCoord::coords(self.edge_length)
	}

	/// Returns all tiles
	pub fn iter(&self) -> impl Iterator<Item = (TileCoord, &TerrainType)> {
		self.playground
			.iter()
			.enumerate()
			.map(|(i, t)| (self.coord(i), t))
	}

	/// Returns all tiles mutably
	pub fn iter_mut(&mut self) -> impl Iterator<Item = (TileCoord, &mut TerrainType)> {
		self.playground
			.iter_mut()
			.enumerate()
			.map(|(i, t)| (coord(self.edge_length, i), t))
	}

	/// The edge length of the map in meters
	pub fn map_size(&self) -> f32 {
		(self.edge_length as u32 * TILE_SIZE) as f32
	}

	/// Returns the coordinates of a random tile
	pub fn random_tile<R: Rng>(&self, mut rng: R) -> TileCoord {
		TileCoord {
			x: rng.gen_range(0..self.edge_length),
			y: rng.gen_range(0..self.edge_length),
		}
	}

	/// Returns an radom location within the map
	pub fn random_location<R: Rng>(&self, mut rng: R) -> Location {
		Location(Vec2::new(
			rng.gen_range(0.0..self.map_size()),
			rng.gen_range(0.0..self.map_size()),
		))
	}

	/// Returns an radom location within the map that is on a passable tile
	pub fn random_passable_location<R: Rng>(&self, mut rng: R) -> Location {
		// Just use rejection sampling
		loop {
			let candidate = self.random_location(&mut rng);

			// Check if the location is on a passable tile
			if self.get(candidate.into()).is_passable() {
				return candidate;
			}
		}
	}
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
	pub trim: Trim,
	/// The current steering as fraction from -1.0 to +1.0.
	///
	/// The meaning is as follows:
	/// * `-1.0` means full deflection towards the left
	/// * `0.0` means neutral, straight ahead
	/// * `+1.0` means full deflection towards the right
	pub rudder: BiPolarFraction,
}
