use enum_map::Enum;
use nalgebra_glm::Vec2;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

use crate::units::Distance;
use crate::units::Elevation;
use crate::units::Location;
use crate::TILE_SIZE;



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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TileCoordOutOfBoundsError {
	UnderRun,
	OverRun,
}
/// Gives the coordinates of the tile below the given location
///
/// Notice, if the location is out-of-bounds of the map, so will the tile coord.
impl TryFrom<Location> for TileCoord {
	type Error = TileCoordOutOfBoundsError;

	fn try_from(loc: Location) -> Result<Self, Self::Error> {
		// TODO: `n > u32::MAX` and `n / TILE_SIZE > u16::MAX` more gracefully

		if loc.0.x < 0.0 || loc.0.y < 0.0 {
			return Err(TileCoordOutOfBoundsError::UnderRun);
		}
		assert!(loc.0.x >= 0.0, "x is negative (or nan)");
		assert!(loc.0.y >= 0.0, "y is negative (or nan)");

		Ok(Self {
			x: (loc.0.x as u32 / TILE_SIZE)
				.try_into()
				.map_err(|_| TileCoordOutOfBoundsError::OverRun)?,
			y: (loc.0.y as u32 / TILE_SIZE)
				.try_into()
				.map_err(|_| TileCoordOutOfBoundsError::OverRun)?,
		})
	}
}
/// Gives the center point of the tile
impl From<TileCoord> for Location {
	fn from(tc: TileCoord) -> Self {
		Self(Vec2::new(
			(tc.x as u32 * TILE_SIZE) as f32 + 0.5 * TILE_SIZE as f32,
			(tc.y as u32 * TILE_SIZE) as f32 + 0.5 * TILE_SIZE as f32,
		))
	}
}

/// The direction of tile connections.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[derive(strum::EnumIter)]
#[derive(Enum)]
pub enum TileDirection {
	East,
	South,
	West,
	North,
}
impl TileDirection {
	/// Gives the tile direction turned clock wise.
	pub const fn turn_cw(self) -> Self {
		match self {
			Self::East => Self::South,
			Self::South => Self::West,
			Self::West => Self::North,
			Self::North => Self::East,
		}
	}

	/// Gives the tile direction turned counter clock wise.
	pub const fn turn_ccw(self) -> Self {
		match self {
			Self::East => Self::North,
			Self::South => Self::East,
			Self::West => Self::South,
			Self::North => Self::West,
		}
	}

	/// Gives the tile offset for going into this direction as `(x, y)` pair.
	#[inline]
	pub const fn tile_offsets(self) -> (i8, i8) {
		match self {
			Self::East => (1, 0),
			Self::South => (0, 1),
			Self::West => (-1, 0),
			Self::North => (0, -1),
		}
	}

	/// Gives the absolute tile Coordinate from `tc` in the direction of `self` wrapping around at the map edge like a torus.
	pub const fn of(self, mut tc: TileCoord, edge_len: u16) -> TileCoord {
		const fn wrapping_inc(a: u16, edge_len: u16) -> u16 {
			if a >= edge_len - 1 {
				0
			} else {
				a + 1
			}
		}
		const fn wrapping_dec(a: u16, edge_len: u16) -> u16 {
			if a == 0 {
				edge_len - 1
			} else {
				a - 1
			}
		}
		const fn apply_offset(a: u16, offset: i8, edge_len: u16) -> u16 {
			match offset {
				-1 => wrapping_dec(a, edge_len),
				0 => a,
				1 => wrapping_inc(a, edge_len),
				_ => panic!("Invalid tile offset"),
			}
		}

		let (x, y) = self.tile_offsets();

		tc.x = apply_offset(tc.x, x, edge_len);
		tc.y = apply_offset(tc.y, y, edge_len);

		tc
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
	pub playground: Vec<Elevation>,
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

	pub const fn tile_in_direction(&self, dir: TileDirection, tc: TileCoord) -> TileCoord {
		dir.of(tc, self.edge_length)
	}

	/// Returns the tile coord west of the given one
	pub fn west_of(&self, tc: TileCoord) -> TileCoord {
		self.tile_in_direction(TileDirection::West, tc)
	}

	/// Returns the tile coord east of the given one
	pub fn east_of(&self, tc: TileCoord) -> TileCoord {
		self.tile_in_direction(TileDirection::East, tc)
	}

	/// Returns the tile coord north of the given one
	pub fn north_of(&self, tc: TileCoord) -> TileCoord {
		self.tile_in_direction(TileDirection::North, tc)
	}

	/// Returns the tile coord south of the given one
	pub fn south_of(&self, tc: TileCoord) -> TileCoord {
		self.tile_in_direction(TileDirection::South, tc)
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

	pub fn try_get(&self, tc: TileCoord) -> Option<&Elevation> {
		if tc.x >= self.edge_length {
			None
		} else {
			self.playground.get(self.index(tc))
		}
	}

	/// Gets tile type at given coordinate
	#[track_caller]
	pub fn get(&self, tc: TileCoord) -> &Elevation {
		let idx = self.index(tc);
		&self.playground[idx]
	}

	/// Gets mutably the tile type at given coordinate
	pub fn get_mut(&mut self, tc: TileCoord) -> &mut Elevation {
		let idx = self.index(tc);
		&mut self.playground[idx]
	}

	/// Creates a terrain from an array of rows.
	///
	/// I.e. a tile at (x,y) would be represented by `array[x][y]`
	pub fn from_array<const N: usize>(array: [[Elevation; N]; N]) -> Self {
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
	pub fn iter(&self) -> impl Iterator<Item = (TileCoord, &Elevation)> {
		self.playground
			.iter()
			.enumerate()
			.map(|(i, t)| (self.coord(i), t))
	}

	/// Returns all tiles mutably
	pub fn iter_mut(&mut self) -> impl Iterator<Item = (TileCoord, &mut Elevation)> {
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
			if self.get(candidate.try_into().unwrap()).is_passable() {
				return candidate;
			}
		}
	}

	/// Returns the corresponding normalized location on the terrain of the give location.
	///
	/// This function essentially calculates the positive modulo of the given location and the size of the terrain.
	pub fn map_loc_on_torus(&self, mut loc: Location) -> Location {
		// Map the location on the Torus-world
		loc.0.x = loc.0.x.rem_euclid(self.map_size());
		loc.0.y = loc.0.y.rem_euclid(self.map_size());
		// Apparently, floating-point rems, may return a value as big as `rhs`
		// So we need to fix that
		// Maybe we could use one day `next_down()` instead
		if loc.0.x == self.map_size() {
			loc.0.x = 0.0;
		}
		if loc.0.y == self.map_size() {
			loc.0.y = 0.0;
		}

		loc
	}

	/// Returns the shortest distance from one location to another on a torus.
	pub fn torus_distance(&self, from: Location, to: Location) -> Distance {
		let from = self.map_loc_on_torus(from);
		let to = self.map_loc_on_torus(to);

		let mut distance = to - from;

		let half_size = self.map_size() / 2.;
		if distance.0.x.abs() > half_size {
			let s = distance.0.x.signum();
			distance.0.x = (self.map_size() - distance.0.x.abs()) * s * -1.;
		}
		if distance.0.y.abs() > half_size {
			let s = distance.0.y.signum();
			distance.0.y = (self.map_size() - distance.0.y.abs()) * s * -1.;
		}

		distance
	}

	/// Returns wether `x` lies between `min` and `max` on a Torus world.
	///
	/// This check is a conventional AABB check if `min` <= `max` (for each
	/// component), it becomes a wrapping check, if `max` < `min`, meaning
	/// that, `x` needs to be outside the conventional AABB.
	pub fn torus_bounds_check(&self, min: Location, max: Location, x: Location) -> bool {
		// First move all points relative to `min`
		let mini_x = Location((x - min).0);
		let mini_max = Location((max - min).0);

		// Remap onto the torus
		let mapped_mini_x = self.map_loc_on_torus(mini_x);
		let mapped_mini_max = self.map_loc_on_torus(mini_max);

		// Just do a conventional AABB check, given that `min` is now the origin.
		mapped_mini_x.0.x < mapped_mini_max.0.x && mapped_mini_x.0.y < mapped_mini_max.0.y
	}

	/// Remaps `x` into the torus starting at `min`
	pub fn torus_remap(&self, min: Location, x: Location) -> Location {
		// First move all points relative to `min`
		let mini_x = Location(x.0 - min.0);

		// Remap onto the torus
		let mapped_mini_x = self.map_loc_on_torus(mini_x);

		// Readd our "origin" point
		Location(mapped_mini_x.0 + min.0)
	}
}
