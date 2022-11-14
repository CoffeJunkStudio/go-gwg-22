use asset_config::AssetConfig;
use good_web_game as gwg;
use good_web_game::graphics::spritebatch::SpriteBatch;
use good_web_game::GameResult;
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
	pub deep: SpriteBatch,
	pub shallow: SpriteBatch,
	pub beach: SpriteBatch,
	pub land: SpriteBatch,
}

/// Asset of one ship
pub struct ShipSprites {
	pub body: AssetBatch,
	pub sail: Vec<AssetBatch>,
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
