use asset_config::AssetConfig;
use enum_map::EnumMap;
use good_web_game as gwg;
use good_web_game::graphics::spritebatch::SpriteBatch;
use good_web_game::GameResult;
use logic::state::SailKind;
use logic::state::ShipHull;
use nalgebra::Point2;

use self::asset_batch::AssetBatch;


pub mod asset_batch;


/// The location of the asset configuration file
const ASSET_CONFIG_STR: &str = include_str!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/asset-repo/render_assets.toml"
));

/// Terrain assets bundle
pub struct TerrainBatches {
	/// Represents the deep ocean
	pub deep: SpriteBatch,
	/// Represents water near shore
	pub shallow: SpriteBatch,
	/// Represents land near water
	pub beach: SpriteBatch,
	/// Represents inward land
	pub land: SpriteBatch,

	/// An animation layer for water waves
	pub water_anim: SpriteBatch,
	/// Second animation layer for water waves
	pub water_anim_2: SpriteBatch,
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
