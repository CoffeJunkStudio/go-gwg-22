use nalgebra_glm::Vec2;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

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
		self.playground.get(self.index(tc))
	}

	/// Gets tile type at given coordinate
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
}
