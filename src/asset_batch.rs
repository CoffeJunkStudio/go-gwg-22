use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::path::PathBuf;

use asset_config::AssetConfig;
use good_web_game as gwg;
use good_web_game::graphics::spritebatch::SpriteBatch;
use good_web_game::graphics::spritebatch::SpriteIdx;
use good_web_game::graphics::Rect;
use gwg::graphics::DrawParam;
use gwg::graphics::{self,};

fn norm_angle(angle: f64) -> f64 {
	angle.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU
}

pub fn image_batch(
	ctx: &mut gwg::Context,
	quad_ctx: &mut gwg::miniquad::Context,
	path: impl AsRef<Path>,
) -> gwg::GameResult<SpriteBatch> {
	let image = graphics::Image::new(ctx, quad_ctx, path)?;
	Ok(graphics::spritebatch::SpriteBatch::new(image))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetParams {
	pub z_local_frames: u32,
	pub z_frames: u32,
	pub x_frames: u32,
	pub width: u32,
	pub height: u32,
}

#[derive(Debug)]
pub struct AssetBatch {
	batch: SpriteBatch,
	params: AssetParams,
}

impl AssetBatch {
	pub fn new(batch: SpriteBatch, params: AssetParams) -> Self {
		Self {
			batch,
			params,
		}
	}

	pub fn from_image_file(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		path: impl AsRef<Path>,
		params: AssetParams,
	) -> gwg::GameResult<Self> {
		let batch = image_batch(ctx, quad_ctx, path)?;
		Ok(Self {
			batch,
			params,
		})
	}

	pub fn from_config(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		config: &AssetConfig,
		asset_name: &str,
	) -> gwg::GameResult<Self> {
		let asset = config.find_asset(asset_name).unwrap();
		let asset_filename = config.get_asset_output(asset_name).unwrap();
		let asset_filepath = PathBuf::from("assets")
			.join("rendered")
			.join(asset_filename);

		let params = AssetParams {
			z_local_frames: asset.z_local_frames,
			z_frames: asset.z_frames,
			x_frames: asset.x_frames,
			width: asset.width,
			height: asset.height.unwrap_or(asset.width),
		};

		Self::from_image_file(ctx, quad_ctx, asset_filepath, params)
	}

	pub fn add_frame(
		&mut self,
		angle_z_local: f64,
		angle_z: f64,
		angle_x: f64,
		into_param: impl Into<DrawParam>,
	) -> SpriteIdx {
		fn compute_offset(frames: u32, angle: f64) -> f32 {
			let anim_progress = norm_angle(angle);
			let frame = ((f64::from(frames - 1) * anim_progress.clamp(0.0, 1.0)).round() as u32)
				.min(frames - 1);
			frame as f32 / frames as f32
		}

		let offs_z_local = compute_offset(self.params.z_local_frames, angle_z_local);
		let offs_z = compute_offset(self.params.z_frames, angle_z);
		let offs_x = compute_offset(
			self.params.x_frames,
			(angle_x + std::f64::consts::FRAC_PI_2) * 2.0,
		);

		let src = Rect {
			x: offs_z,
			y: offs_z_local + offs_x / self.params.z_local_frames as f32,
			w: 1.0 / self.params.z_frames as f32,
			h: 1.0 / self.params.x_frames as f32 / self.params.z_local_frames as f32,
		};
		let param = into_param.into().src(src);
		self.batch.add(param)
	}

	pub const fn params(&self) -> &AssetParams {
		&self.params
	}
}

impl AsRef<SpriteBatch> for AssetBatch {
	fn as_ref(&self) -> &SpriteBatch {
		&self.batch
	}
}

impl AsMut<SpriteBatch> for AssetBatch {
	fn as_mut(&mut self) -> &mut SpriteBatch {
		&mut self.batch
	}
}

impl Deref for AssetBatch {
	type Target = SpriteBatch;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl DerefMut for AssetBatch {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}
