use asset_config::AssetConfig;
use enum_map::EnumMap;
use good_web_game as gwg;
use good_web_game::graphics::spritebatch::SpriteBatch;
use good_web_game::GameResult;
use gwg::graphics::Color;
use gwg::graphics::Image;
use logic::state::SailKind;
use logic::state::ShipHull;
use logic::units::TileType;
use nalgebra::Point2;

use self::asset_batch::AssetBatch;

pub mod asset_batch;

/// The location of the asset configuration file
const ASSET_CONFIG_STR: &str = include_str!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/asset-repo/render_assets.toml"
));

/// UI assets bundle
pub struct UiImages {
	/// Image to indicate the direction of the wind
	pub wind_direction_indicator: Image,
	pub wind_speed_colors: Vec<Color>,
}

/// Terrain assets bundle
pub struct TerrainBatches {
	/// Represents the deep ocean
	pub deep: SpriteBatch,
	/// Represents water near shore
	pub shallow: SpriteBatch,
	/// Represents land near water
	pub beach: SpriteBatch,
	/// Represents inward land
	pub grass: SpriteBatch,

	/// Solid mask for shallow water
	pub shallow_solid: SpriteBatch,
	/// Shallow to deep water corner transition mask
	pub shallow_c1: SpriteBatch,
	/// Shallow to deep water 1-side transition mask
	pub shallow_s1: SpriteBatch,
	/// Shallow to deep water 2-side transition mask
	pub shallow_s2: SpriteBatch,
	/// Shallow to deep water 3-side transition mask
	pub shallow_s3: SpriteBatch,
	/// Shallow to deep water 4-side transition mask
	pub shallow_s4: SpriteBatch,

	/// Solid mask for beach
	pub beach_solid: SpriteBatch,
	/// Beach to water corner transition mask
	pub beach_c1: SpriteBatch,
	/// Beach to water 1-side transition mask
	pub beach_s1: SpriteBatch,
	/// Beach to water 2-side transition mask
	pub beach_s2: SpriteBatch,
	/// Beach to water 3-side transition mask
	pub beach_s3: SpriteBatch,
	/// Beach to water 4-side transition mask
	pub beach_s4: SpriteBatch,

	/// Solid mask for grass
	pub grass_solid: SpriteBatch,
	/// Grass to others corner transition mask
	pub grass_c1: SpriteBatch,
	/// Grass to others 1-side transition mask
	pub grass_s1: SpriteBatch,
	/// Grass to others 2-side transition mask
	pub grass_s2: SpriteBatch,
	/// Grass to others 3-side transition mask
	pub grass_s3: SpriteBatch,
	/// Grass to others 4-side transition mask
	pub grass_s4: SpriteBatch,

	/// An animation layer for water waves
	pub water_anim: SpriteBatch,
	/// Second animation layer for water waves
	pub water_anim_2: SpriteBatch,
}

impl TerrainBatches {
	/// Gives the sprite for the tile of the given type
	pub fn tile_sprite(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => &mut self.deep,
			TileType::ShallowWater => &mut self.shallow,
			TileType::Beach => &mut self.beach,
			TileType::Grass => &mut self.grass,
		}
	}

	/// Gives the solid mask for the tile of the given type
	pub fn tile_mask_solid(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => unimplemented!("There are not masks for Deep Water"),
			TileType::ShallowWater => &mut self.shallow_solid,
			TileType::Beach => &mut self.beach_solid,
			TileType::Grass => &mut self.grass_solid,
		}
	}

	/// Gives the corner mask for the tile of the given type
	pub fn tile_mask_c1(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => unimplemented!("There are not masks for Deep Water"),
			TileType::ShallowWater => &mut self.shallow_c1,
			TileType::Beach => &mut self.beach_c1,
			TileType::Grass => &mut self.grass_c1,
		}
	}

	/// Gives the single side mask for the tile of the given type
	pub fn tile_mask_s1(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => unimplemented!("There are not masks for Deep Water"),
			TileType::ShallowWater => &mut self.shallow_s1,
			TileType::Beach => &mut self.beach_s1,
			TileType::Grass => &mut self.grass_s1,
		}
	}

	/// Gives the single side mask for the tile of the given type
	pub fn tile_mask_s2(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => unimplemented!("There are not masks for Deep Water"),
			TileType::ShallowWater => &mut self.shallow_s2,
			TileType::Beach => &mut self.beach_s2,
			TileType::Grass => &mut self.grass_s2,
		}
	}

	/// Gives the single side mask for the tile of the given type
	pub fn tile_mask_s3(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => unimplemented!("There are not masks for Deep Water"),
			TileType::ShallowWater => &mut self.shallow_s3,
			TileType::Beach => &mut self.beach_s3,
			TileType::Grass => &mut self.grass_s3,
		}
	}

	/// Gives the single side mask for the tile of the given type
	pub fn tile_mask_s4(&mut self, tt: TileType) -> &mut SpriteBatch {
		match tt {
			TileType::DeepWater => unimplemented!("There are not masks for Deep Water"),
			TileType::ShallowWater => &mut self.shallow_s4,
			TileType::Beach => &mut self.beach_s4,
			TileType::Grass => &mut self.grass_s4,
		}
	}

	/// Returns all sprite batches for shallow water
	pub fn shallow_batches(&mut self) -> (&mut SpriteBatch, Vec<&mut SpriteBatch>) {
		(
			&mut self.shallow,
			vec![
				&mut self.shallow_solid,
				&mut self.shallow_c1,
				&mut self.shallow_s1,
				&mut self.shallow_s2,
				&mut self.shallow_s3,
				&mut self.shallow_s4,
			],
		)
	}

	/// Returns all sprite batches for beach
	pub fn beach_batches(&mut self) -> (&mut SpriteBatch, Vec<&mut SpriteBatch>) {
		(
			&mut self.beach,
			vec![
				&mut self.beach_solid,
				&mut self.beach_c1,
				&mut self.beach_s1,
				&mut self.beach_s2,
				&mut self.beach_s3,
				&mut self.beach_s4,
			],
		)
	}

	/// Returns all sprite batches for grass
	pub fn grass_batches(&mut self) -> (&mut SpriteBatch, Vec<&mut SpriteBatch>) {
		(
			&mut self.grass,
			vec![
				&mut self.grass_solid,
				&mut self.grass_c1,
				&mut self.grass_s1,
				&mut self.grass_s2,
				&mut self.grass_s3,
				&mut self.grass_s4,
			],
		)
	}
}

/// Asset of one ship
pub struct ShipSprites {
	pub body: EnumMap<ShipHull, AssetBatch>,
	pub sail: EnumMap<SailKind, Vec<AssetBatch>>,
}

/// Ship asset bundle
pub struct ShipBatches {
	pub basic: ShipSprites,
}

/// Map resource asset bundle
pub struct ResourceBatches {
	pub fishes: Vec<AssetBatch>,
	pub starfishes: Vec<AssetBatch>,
	pub shoe: Vec<AssetBatch>,
	pub grass: Vec<AssetBatch>,
}

/// Map building asset bundle
pub struct BuildingBatches {
	pub harbor: AssetBatch,
}

/// Load the asset configuration file
pub fn load_asset_config() -> AssetConfig {
	toml::from_str(ASSET_CONFIG_STR).unwrap()
}

/// Dispatch the draw calls of all given sprite batches and clears them
pub fn draw_and_clear<'a>(
	ctx: &mut gwg::Context,
	quad_ctx: &mut gwg::miniquad::Context,
	batches: impl IntoIterator<Item = &'a mut SpriteBatch>,
) -> GameResult<()> {
	for batch in batches {
		// For some ridiculous reason, empty sprite batches cause sever glitches (UB-like) on windows.
		// Thus we will only draw those that aren't empty.
		if !batch.get_sprites().is_empty() {
			gwg::graphics::draw(ctx, quad_ctx, batch, (Point2::new(0.0, 0.0),))?;
			batch.clear();
		}
	}

	Ok(())
}
