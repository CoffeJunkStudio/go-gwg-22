use std::ops::DerefMut;
use std::path::Path;

use cfg_if::cfg_if;
use enum_map::enum_map;
use good_web_game as gwg;
use gwg::GameResult;
use gwg::audio;
use gwg::cgmath::Point2;
use gwg::goodies::scene::Scene;
use gwg::goodies::scene::SceneSwitch;
use gwg::graphics;
use gwg::graphics::BlendMode;
use gwg::graphics::draw;
use gwg::graphics::Canvas;
use gwg::graphics::Color;
use gwg::graphics::DrawMode;
use gwg::graphics::DrawParam;
use gwg::graphics::Drawable;
use gwg::graphics::Image;
use gwg::graphics::MeshBuilder;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::StrokeOptions;
use gwg::graphics::Text;
use gwg::graphics::Transform;
use gwg::graphics::spritebatch::SpriteBatch;
use gwg::miniquad::KeyCode;
use gwg::timer;
use logic::generator::Generator;
use logic::generator::PerlinNoise;
use logic::generator::Setting;
use logic::generator::WhiteNoise;
use logic::glm::vec1;
use logic::glm::Vec2;
use logic::state::Event;
use logic::state::SailKind;
use logic::terrain::TileCoord;
use logic::units::BiPolarFraction;
use logic::units::Distance;
use logic::units::Location;
use logic::units::TileType;
use logic::Input;
use logic::World;
use logic::TICKS_PER_SECOND;
use logic::TILE_SIZE;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use wyhash::wyhash;

use super::GlobalState;
use crate::assets::asset_batch::image_batch;
use crate::assets::asset_batch::AssetBatch;
use crate::assets::draw_and_clear;
use crate::assets::load_asset_config;
use crate::assets::BuildingBatches;
use crate::assets::ResourceBatches;
use crate::assets::ShipBatches;
use crate::assets::ShipSprites;
use crate::assets::TerrainBatches;
use crate::assets::UiImages;
use crate::draw_version;

/// Zoom factor exponentiation base.
///
/// Also see: [Game::zoom_factor_exp]
const ZOOM_FACTOR_BASE: f32 = std::f32::consts::SQRT_2;

/// The amount of the world visible across the screen diagonal (i.e. the windows diagonal).
///
/// See: [Game::pixel_per_meter]
const METERS_PER_SCREEN_DIAGONAL: f32 = 30.;

/// The default (i.e. initial) zoom factor exponent
///
/// Also see: [Game::zoom_factor_exp]
const DEFAULT_ZOOM_LEVEL: i32 = -2;

trait Mix {
	fn mix(&self, other: &Self, mix_factor: f32) -> Self;
}

impl Mix for Color {
	fn mix(&self, other: &Self, mix_factor: f32) -> Self {
		Self::new(
			(1.0 - mix_factor) * self.r + mix_factor * other.r,
			(1.0 - mix_factor) * self.g + mix_factor * other.g,
			(1.0 - mix_factor) * self.b + mix_factor * other.b,
			(1.0 - mix_factor) * self.a + mix_factor * other.a,
		)
	}
}

pub struct Images {
	terrain_batches: TerrainBatches,
	ship_batches: ShipBatches,
	resource_batches: ResourceBatches,
	building_batches: BuildingBatches,
	ui: UiImages,
}

// #[derive(Debug)] `audio::Source` dose not implement Debug!
struct Audios {
	sound_enabled: bool,
	music_enabled: bool,
	sound: audio::Source,
	fail_sound: audio::Source,
	sell_sound: audio::Source,
	upgrade_sound: audio::Source,
	sound_fishy_1: audio::Source,
	sound_fishy_2: audio::Source,
	sound_fishy_3: audio::Source,
	music_0: audio::Source,
	water_sound_0: audio::Source,
	water_sound_1: audio::Source,
}

// #[derive(Debug)] `audio::Source` dose not implement Debug!
pub struct Game {
	/// The drawables
	images: Images,
	/// The audio files
	audios: Audios,

	terrain_transition_canvas: Canvas,
	terrain_transition_mask_canvas: Canvas,

	full_screen: bool,
	world: World,
	input: Input,
	/// The exponent to calculate the zoom factor
	///
	/// The bigger this value, the more pixel a meter is on the screen (i.e. more zoomed in).
	///
	/// See: [Game::pixel_per_meter]
	zoom_factor_exp: i32,
	/// Offset of the water waves within a tile
	water_wave_offset: Vec2,
	/// Offset of the secondary water waves within a tile
	water_wave_2_offset: Vec2,
}

impl Game {
	pub fn new(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::GraphicsContext,
	) -> gwg::GameResult<Self> {
		let opts = &*crate::OPTIONS;

		let seed: u64 = opts
			.seed
			.as_ref()
			.map(|s| wyhash(s.as_bytes(), 0))
			.unwrap_or(gwg::timer::time().floor() as u64);

		let sound_enabled = !opts.muted;
		cfg_if! {
			if #[cfg(target_family = "wasm")] {
				// TODO: without a main menu, we need first user-input before
				// we can enable music, thus we disable it so it is enablable.
				let music_enabled = false;
			} else {
				let music_enabled = !opts.muted;
			}
		}

		println!(
			"{:.3} [game] loading music...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let mut music_0 = audio::Source::new(ctx, "/music/sailing-chanty.ogg")?;
		music_0.set_repeat(true);
		if music_enabled {
			music_0.play(ctx)?;
		}

		println!(
			"{:.3} [game] loading sounds...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;
		let fail_sound = audio::Source::new(ctx, "/sound/invalid.ogg")?;
		let upgrade_sound = audio::Source::new(ctx, "/sound/upgrade.ogg")?;
		let sound_fishy_1 = audio::Source::new(ctx, "/sound/fischie.ogg")?;
		let sound_fishy_2 = audio::Source::new(ctx, "/sound/fischie2.ogg")?;
		let sound_fishy_3 = audio::Source::new(ctx, "/sound/fischie3.ogg")?;

		let mut sell_sound = audio::Source::new(ctx, "/sound/sell-sound.ogg")?;
		sell_sound.set_repeat(true);
		let mut water_sound_0 = audio::Source::new(ctx, "/sound/waterssoftloop.ogg")?;
		water_sound_0.set_repeat(true);
		let mut water_sound_1 = audio::Source::new(ctx, "/sound/waterstrongloop.ogg")?;
		water_sound_1.set_repeat(true);
		if sound_enabled {
			sell_sound.set_volume(ctx, 0.)?;
			sell_sound.play(ctx)?;
			water_sound_1.set_volume(ctx, 0.)?;
			water_sound_1.play(ctx)?;
		}
		if music_enabled {
			water_sound_0.play(ctx)?;
		}

		println!(
			"{:.3} [game] loading config...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let render_config = load_asset_config();

		println!(
			"{:.3} [game] loading terrain...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let terrain_batches = TerrainBatches {
			deep: image_batch(ctx, quad_ctx, "img/deepwater0.png")?,
			shallow: image_batch(ctx, quad_ctx, "img/shallowwater.png")?,
			beach: image_batch(ctx, quad_ctx, "img/sand.png")?,
			grass: image_batch(ctx, quad_ctx, "img/grass.png")?,

			shallow_c1: image_batch(ctx, quad_ctx, "img/mask_shallow_c1.png")?,
			shallow_s1: image_batch(ctx, quad_ctx, "img/mask_shallow_s1.png")?,
			shallow_s2: image_batch(ctx, quad_ctx, "img/mask_shallow_s2.png")?,
			shallow_s3: image_batch(ctx, quad_ctx, "img/mask_shallow_s3.png")?,
			shallow_s4: image_batch(ctx, quad_ctx, "img/mask_shallow_s4.png")?,

			beach_c1: image_batch(ctx, quad_ctx, "img/mask_sand_c1.png")?,
			beach_s1: image_batch(ctx, quad_ctx, "img/mask_sand_s1.png")?,
			beach_s2: image_batch(ctx, quad_ctx, "img/mask_sand_s2.png")?,
			beach_s3: image_batch(ctx, quad_ctx, "img/mask_sand_s3.png")?,
			beach_s4: image_batch(ctx, quad_ctx, "img/mask_sand_s4.png")?,

			grass_c1: image_batch(ctx, quad_ctx, "img/mask_grass_c1.png")?,
			grass_s1: image_batch(ctx, quad_ctx, "img/mask_grass_s1.png")?,
			grass_s2: image_batch(ctx, quad_ctx, "img/mask_grass_s2.png")?,
			grass_s3: image_batch(ctx, quad_ctx, "img/mask_grass_s3.png")?,
			grass_s4: image_batch(ctx, quad_ctx, "img/mask_grass_s4.png")?,

			water_anim: image_batch(ctx, quad_ctx, "img/wateranim.png")?,
			water_anim_2: image_batch(ctx, quad_ctx, "img/wateranim2.png")?,
		};

		println!(
			"{:.3} [game] loading ships...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let ship_batches = ShipBatches {
			basic: ShipSprites {
				body: enum_map! {
					logic::state::ShipHull::Small => AssetBatch::from_config(ctx, quad_ctx, &render_config, "ship-00")?,
					logic::state::ShipHull::Bigger => AssetBatch::from_config(ctx, quad_ctx, &render_config, "ship-01")?,
				},
				sail: enum_map! {
					logic::state::SailKind::Cog => vec![
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-02-0")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-02-1")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-02-2")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-02-3")?,
				],
				logic::state::SailKind::Bermuda => vec![
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-0")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-1")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-2")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-3")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-4")?,
				],
				logic::state::SailKind::Schooner => vec![
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-0")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-1")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-2")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-3")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-4")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-5")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-6")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-01-7")?,
				]
				},
			},
		};

		println!(
			"{:.3} [game] loading resources...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let resource_batches = ResourceBatches {
			fishes: [
				"fish-00", "fish-01", "fish-02", "fish-03", "fish-04", "fish-05", "fish-06",
				"fish-07",
			]
			.into_iter()
			.map(|name| AssetBatch::from_config(ctx, quad_ctx, &render_config, name).unwrap())
			.collect(),
		};

		println!(
			"{:.3} [game] loading buildings...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let building_batches = BuildingBatches {
			harbor: AssetBatch::from_config(ctx, quad_ctx, &render_config, "harbour-00").unwrap(),
		};

		println!(
			"{:.3} [game] loading ui...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let ui = UiImages {
			wind_direction_indicator: Image::new(ctx, quad_ctx, Path::new("img/wind-arrow.png"))
				.unwrap(),
			wind_speed_colors: vec![Color::GREEN, Color::YELLOW, Color::RED],
		};

		println!(
			"{:.3} [game] loading other stuff...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let terrain_transition_canvas = Canvas::with_window_size(ctx, quad_ctx)?;
		let terrain_transition_mask_canvas = Canvas::with_window_size(ctx, quad_ctx)?;

		println!(
			"{:.3} [game] generating world...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		// Generate world
		let noise = WhiteNoise;
		let resource_density = {
			cfg_if! {
				if #[cfg(feature = "dev")] {
					opts.resource_factor_cheat.unwrap_or(1.0)
				} else {
					1.0
				}
			}
		};
		let settings = Setting {
			edge_length: opts.map_size,
			resource_density,
		};

		let mut rng = logic::StdRng::new(0xcafef00dd15ea5e5, seed.into());
		let mut world = noise.generate(&settings, &mut rng);
		world.state.player.vehicle.heading = 1.0;
		world.state.player.vehicle.pos = world.init.terrain.random_passable_location(&mut rng);
		cfg_if! {
			if #[cfg(feature = "dev")] {
				if let Some(money) = opts.money_cheat {
					world.state.player.money = money;
				}
			}
		}
		world.init.dbg = crate::OPTIONS.to_debugging_conf();

		let s = Game {
			images: Images {
				terrain_batches,
				ship_batches,
				resource_batches,
				building_batches,
				ui,
			},
			audios: Audios {
				sound_enabled,
				music_enabled,
				sound,
				fail_sound,
				sell_sound,
				upgrade_sound,
				sound_fishy_1,
				sound_fishy_2,
				sound_fishy_3,
				music_0,
				water_sound_0,
				water_sound_1,
			},
			terrain_transition_canvas,
			terrain_transition_mask_canvas,
			full_screen: false,
			world,
			input: Input::default(),
			zoom_factor_exp: DEFAULT_ZOOM_LEVEL,
			water_wave_offset: Default::default(),
			water_wave_2_offset: Default::default(),
		};

		println!(
			"{:.3} [game] ready to go",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);

		Ok(s)
	}

	/// A unitless factor for zooming the game view
	///
	/// The bigger this factor, the more pixels a meter is on the screen (i.e. zoomed in).
	fn zoom_factor(&self) -> f32 {
		ZOOM_FACTOR_BASE.powi(self.zoom_factor_exp)
	}

	/// Conversion factor between world meter and screen pixel.
	fn pixel_per_meter(&self, ctx: &gwg::Context) -> f32 {
		// Get the current screen size
		let Rect {
			w,
			h,
			..
		} = gwg::graphics::screen_coordinates(ctx);
		// px/diag
		let diag_size = (w * w + h * h).sqrt();

		// in m/diag
		let m_p_sd = METERS_PER_SCREEN_DIAGONAL;

		// in px/m
		let meter_res = diag_size / m_p_sd;

		meter_res * self.zoom_factor()
	}

	fn draw_text_with_halo(
		&self,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		text: &Text,
		params: impl Into<DrawParam>,
		halo_color: Color,
	) -> gwg::GameResult<()> {
		let params = params.into();

		let mut halo_params = params;
		halo_params.color = halo_color;

		let base_matrix: logic::glm::Mat4 = params.trans.to_bare_matrix().into();

		let offset_param = |offset: Point2<f32>| {
			let glm_matrix: logic::glm::Mat4 =
				DrawParam::from((offset,)).trans.to_bare_matrix().into();
			let new_matrix = base_matrix * glm_matrix;

			DrawParam {
				trans: Transform::Matrix(new_matrix.into()),
				color: halo_color,
				..Default::default()
			}
		};

		graphics::draw(ctx, quad_ctx, text, offset_param(Point2::new(-1., -1.)))?;
		graphics::draw(ctx, quad_ctx, text, offset_param(Point2::new(1., -1.)))?;
		graphics::draw(ctx, quad_ctx, text, offset_param(Point2::new(-1., 1.)))?;
		graphics::draw(ctx, quad_ctx, text, offset_param(Point2::new(1., 1.)))?;

		graphics::draw(ctx, quad_ctx, text, params)?;

		Ok(())
	}

	fn location_to_screen_coords(
		&self,
		ctx: &gwg::Context,
		pos: Location,
	) -> nalgebra::Point2<f32> {
		let screen_coords = gwg::graphics::screen_coordinates(ctx);
		let loc = pos - self.world.state.player.vehicle.pos;
		let sprite_pos = loc.0 * self.pixel_per_meter(ctx)
			+ logic::glm::vec2(screen_coords.w, screen_coords.h) * 0.5;

		nalgebra::Point2::new(sprite_pos.x, sprite_pos.y)
	}

	fn draw_debugging(
		&self,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		let pixel_per_meter = self.pixel_per_meter(ctx);

		if crate::OPTIONS.bounding_boxes {
			// Harbor bounding box
			let mesh = {
				let mut mb = MeshBuilder::new();

				for h in &self.world.state.harbors {
					mb.circle(
						DrawMode::Stroke(StrokeOptions::DEFAULT),
						self.location_to_screen_coords(ctx, h.loc),
						0.5 * logic::HARBOR_SIZE * pixel_per_meter,
						1.0,
						Color::MAGENTA,
					)?;
				}

				mb.build(ctx, quad_ctx)?
			};
			draw(ctx, quad_ctx, &mesh, (Point2::new(0., 0.),))?;

			// Fishies bounding box
			let mesh = {
				let mut mb = MeshBuilder::new();

				for r in &self.world.state.resources {
					mb.circle(
						DrawMode::Stroke(StrokeOptions::DEFAULT),
						self.location_to_screen_coords(ctx, r.loc),
						0.5 * logic::RESOURCE_PACK_FISH_SIZE * pixel_per_meter,
						1.0,
						Color::MAGENTA,
					)?;
				}

				mb.build(ctx, quad_ctx)?
			};
			draw(ctx, quad_ctx, &mesh, (Point2::new(0., 0.),))?;

			// Ship bounding box
			let mesh = MeshBuilder::new()
				.circle(
					DrawMode::Stroke(StrokeOptions::DEFAULT),
					self.location_to_screen_coords(ctx, self.world.state.player.vehicle.pos),
					0.5 * logic::VEHICLE_SIZE * pixel_per_meter,
					1.0,
					Color::MAGENTA,
				)?
				.build(ctx, quad_ctx)?;
			draw(ctx, quad_ctx, &mesh, (Point2::new(0., 0.),))?;

			// Ship's tile bounding box
			let player_tile = TileCoord::try_from(self.world.state.player.vehicle.pos).unwrap();
			let player_tile_loc = Location::from(player_tile);
			let player_tile_top_left = self.location_to_screen_coords(
				ctx,
				Location(player_tile_loc.0 - vec1(TILE_SIZE as f32 * 0.5).xx()),
			);
			let player_tile_bottom_right = self.location_to_screen_coords(
				ctx,
				Location(player_tile_loc.0 + vec1(TILE_SIZE as f32 * 0.5).xx()),
			);
			let rect = Rect::new(
				player_tile_top_left.x,
				player_tile_top_left.y,
				player_tile_bottom_right.x - player_tile_top_left.x,
				player_tile_bottom_right.y - player_tile_top_left.y,
			);
			let mesh = MeshBuilder::new()
				.rectangle(
					DrawMode::Stroke(StrokeOptions::DEFAULT),
					rect,
					Color::MAGENTA,
				)?
				.build(ctx, quad_ctx)?;
			draw(ctx, quad_ctx, &mesh, (Point2::new(0., 0.),))?;
		}

		Ok(())
	}
}

impl Scene<GlobalState> for Game {
	fn name(&self) -> &str {
		"In-Game"
	}

	fn update(
		&mut self,
		_glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
	) -> SceneSwitch<GlobalState> {
		use gwg::input::keyboard::is_key_pressed;

		let opts = &*crate::OPTIONS;

		let mut rng = wyhash::WyRng::seed_from_u64((gwg::timer::time() * 1000.) as u64);

		while gwg::timer::check_update_time(ctx, TICKS_PER_SECOND.into()) {
			let mut rudder = 0.0;

			// Rudder input
			if is_key_pressed(ctx, KeyCode::Left) {
				rudder -= 1.0;
			}
			if is_key_pressed(ctx, KeyCode::Right) {
				rudder += 1.0;
			}

			self.input.rudder = BiPolarFraction::from_f32(rudder).unwrap();
			let events = self.world.state.update(&self.world.init, &self.input);

			// Play event sounds
			if self.audios.sound_enabled {
				for ev in events {
					match ev {
						Event::Fishy => {
							let fishies = [
								&self.audios.sound_fishy_1,
								&self.audios.sound_fishy_2,
								&self.audios.sound_fishy_3,
							];
							let sound = fishies.choose(&mut rng).unwrap();

							sound.play(ctx).unwrap();
						},
					}
				}
			}
		}

		let mut did_trade_successful = false;
		if let Some(mut trade) = self.world.state.get_trading() {
			if is_key_pressed(ctx, KeyCode::S) {
				let res = trade.sell_fish(1);
				if let Some(proceeds) = res {
					if proceeds > 0 {
						did_trade_successful = true;
					} else {
						if self.audios.sound_enabled {
							self.audios.fail_sound.play(ctx).unwrap();
						}
					}
				}
			}
		}
		self.audios
			.sell_sound
			.set_volume(ctx, did_trade_successful as u8 as f32)
			.unwrap();

		// Water wave sound
		let water_per_wind_speed = 1. / 2.;
		let relative_water_seed = {
			let water_seed = self.world.state.wind.0 * water_per_wind_speed;
			let ship_seed = self.world.state.player.vehicle.velocity;
			ship_seed.metric_distance(&water_seed)
		};
		let normalized_rel_water_speed = {
			(relative_water_seed / (2. * logic::MAX_WIND_SPEED * water_per_wind_speed))
				.clamp(0., 1.)
				.powi(2)
		};
		self.audios
			.water_sound_1
			.set_volume(ctx, normalized_rel_water_speed * 2.)
			.unwrap();

		if is_key_pressed(ctx, KeyCode::Escape) {
			SceneSwitch::Pop
		} else {
			SceneSwitch::None
		}
	}

	fn draw(
		&mut self,
		glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		let elapsed = gwg::timer::time_since_start(ctx).as_secs_f32();

		let player_pos = self.world.state.player.vehicle.pos;
		let screen_coords = gwg::graphics::screen_coordinates(ctx);
		let pixel_per_meter = self.pixel_per_meter(ctx);

		// Clear screen
		let red = elapsed.sin() * 0.5 + 0.5;
		let green = (1.3 + elapsed + 0.3).sin() * 0.5 + 0.5;
		let blue = (1.13 * elapsed + 0.7).sin() * 0.5 + 0.5;
		gwg::graphics::clear(ctx, quad_ctx, [red, green, blue, 1.0].into());

		// Tile sizes
		let tile_image_size = 64.;
		let tile_anim_image_size = 64.;

		let full_tile = logic::glm::vec1(logic::TILE_SIZE as f32).xx();
		let half_tile = full_tile * 0.5;
		// Quarter tile size, but going right and up for better visuals
		let quarter_tile = logic::glm::vec2(
			logic::TILE_SIZE as f32 * 0.25,
			logic::TILE_SIZE as f32 * -0.25,
		);

		let terrain = &self.world.init.terrain;

		// Calculate the top left and bottom right corner where to start and stop drawing the tiles.
		let (left_top, right_bottom) = {
			let scm_x = (screen_coords.w / pixel_per_meter)
				.min(terrain.map_size() - 3. * logic::TILE_SIZE as f32);
			let scm_y = (screen_coords.h / pixel_per_meter)
				.min(terrain.map_size() - 3. * logic::TILE_SIZE as f32);
			let dst = Distance::new(scm_x * 0.5, scm_y * 0.5);

			let lt = player_pos - dst - Distance(full_tile);
			let rb = player_pos + dst + Distance(full_tile);

			(lt, rb)
		};

		// Water wave animation, adding half the wind to the offset
		self.water_wave_offset += self.world.state.wind.0 * timer::delta(ctx).as_secs_f32() / 4.;
		// Modulo the waves by tile size
		self.water_wave_offset.x %= TILE_SIZE as f32;
		self.water_wave_offset.y %= TILE_SIZE as f32;

		// Secondary water wave animation, adding half the wind to the offset
		self.water_wave_2_offset +=
			self.world.state.wind.0 * timer::delta(ctx).as_secs_f32() * 2. / 3.;
		// Modulo the waves by tile size
		self.water_wave_2_offset.x %= TILE_SIZE as f32;
		self.water_wave_2_offset.y %= TILE_SIZE as f32;

		// Draw the waves (notice the draw order is given way below via the `draw_and_clear`
		// TODO: draw the wave in wave size i.e. twice the size of a tile.
		for (tc, _tile) in terrain.iter() {
			if terrain.torus_bounds_check(left_top, right_bottom, tc.to_location()) {
				let remapped = terrain.torus_remap(left_top, tc.to_location());

				let scale = logic::TILE_SIZE as f32 * pixel_per_meter / tile_anim_image_size;

				let loc = remapped.0 - half_tile;

				// Add the offset
				let wave_1 = loc + self.water_wave_offset;

				let f1 = (timer::time() * 0.5).sin().powi(6) as f32 * 0.8 + 0.2;
				let f2 = (timer::time() * 0.5).cos().powi(6) as f32 * 0.8 + 0.2;

				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(wave_1)))
					.scale(logic::glm::vec2(scale, scale))
					.color(Color::new(f1, f1, f1, 1.));
				self.images.terrain_batches.water_anim.add(param);

				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(wave_1 - quarter_tile)))
					.scale(logic::glm::vec2(scale, scale))
					.color(Color::new(f2, f2, f2, 1.));
				self.images.terrain_batches.water_anim.add(param);

				// Add the offset
				let wave_2 = loc + self.water_wave_2_offset;

				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(wave_2)))
					.scale(logic::glm::vec2(scale, scale));
				self.images.terrain_batches.water_anim_2.add(param);
			}
		}

		// Draw the tile background
		for (tc, tile) in terrain.iter() {
			if terrain.torus_bounds_check(left_top, right_bottom, tc.to_location()) {
				let remapped = terrain.torus_remap(left_top, tc.to_location());

				let scale = logic::TILE_SIZE as f32 * pixel_per_meter / tile_image_size;
				let loc = remapped.0 - half_tile;
				let dest = self.location_to_screen_coords(ctx, Location(loc));

				/*
				let rel = match tile.classify() {
					TileType::DeepWater => tile.relative_height(),
					TileType::ShallowWater => tile.relative_height() * 0.5 + 0.5,
					TileType::Beach => 1.0,
					TileType::Grass => 1.0,
				}; */
				let rel = 1.0_f32;

				let batch = match tile.classify() {
					TileType::DeepWater => &mut self.images.terrain_batches.deep,
					TileType::ShallowWater => &mut self.images.terrain_batches.shallow,
					TileType::Beach => &mut self.images.terrain_batches.beach,
					TileType::Grass => &mut self.images.terrain_batches.grass,
				};

				let c = 0.5 + 0.5 * rel.clamp(0., 1.);

				let param = DrawParam::new()
					.dest(dest)
					.scale(logic::glm::vec2(scale, scale))
					.color(Color::new(c, c, c, 1.));

				batch.add(param);
			}
		}

		let ship_pos = self.world.state.player.vehicle.pos.0
			- logic::glm::vec1(2.5 * logic::VEHICLE_SIZE).xx() * 0.5;
		let ship_screen_loc = self.location_to_screen_coords(ctx, Location(ship_pos));

		let body = &mut self.images.ship_batches.basic.body[self.world.state.player.vehicle.hull];

		// Draw the player ship
		let ship_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE * pixel_per_meter / body.params().width as f32,
		)
		.xx();
		let param = DrawParam::new().dest(ship_screen_loc).scale(ship_scale);
		let heading = f64::from(self.world.state.player.vehicle.heading);
		let ship_heading = -heading + std::f64::consts::PI;
		body.add_frame(
			0.0,
			ship_heading,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			param,
		);

		// Draw the player sail
		let sail_reefing = self.world.state.player.vehicle.sail.reefing.value();

		let sail_kind = self.world.state.player.vehicle.sail.kind;
		let sail = &mut self.images.ship_batches.basic.sail[sail_kind];
		let max_sail = sail.len() - 1;
		let effective_reefing = usize::from(sail_reefing).min(max_sail);

		let sail_ass = &mut sail[effective_reefing];
		let sail_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE * pixel_per_meter / sail_ass.params().width as f32,
		)
		.xx();
		let sail_param = DrawParam::new().dest(ship_screen_loc).scale(sail_scale);

		let sail_orient = match sail_kind {
			SailKind::Cog => -f64::from(self.world.state.player.vehicle.sail.orientation_rectangle),
			SailKind::Bermuda | SailKind::Schooner => {
				-f64::from(self.world.state.player.vehicle.sail.orientation_triangle)
					+ std::f64::consts::PI
			},
		};

		let sail_ass = &mut sail[effective_reefing];
		sail_ass.add_frame(
			// We need the sail orientation, minus the heading (because the model is in a rotating frame), plus a half turn (because the model is half way turned around).
			sail_orient - ship_heading + std::f64::consts::PI,
			ship_heading,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			sail_param,
		);

		// Draw the resources (i.e. fishys)
		for resource in &self.world.state.resources {
			if terrain.torus_bounds_check(left_top, right_bottom, resource.loc) {
				let remapped = terrain.torus_remap(left_top, resource.loc);

				let resource_pos =
					remapped.0 - logic::glm::vec1(logic::RESOURCE_PACK_FISH_SIZE).xx() * 0.5;
				let dest = self.location_to_screen_coords(ctx, Location(resource_pos));

				let batch = &mut self.images.resource_batches.fishes[usize::from(resource.variant)];

				let resource_scale = logic::glm::vec1(
					logic::RESOURCE_PACK_FISH_SIZE * pixel_per_meter / batch.params().width as f32,
				)
				.xx();

				let depth = ((resource.params.0 + 10) as f32 / 10.).clamp(0., 1.);
				let d_color = depth;
				let d_alpha = (depth * 2. / 3.) + 0.2;

				let param = DrawParam::new()
					.dest(dest)
					.scale(resource_scale)
					.color(Color::new(d_color, d_color, d_color, d_alpha));

				batch.add_frame(0.0, -f64::from(resource.ori), 0.0, param);
			}
		}

		// Draw harbors
		for harbor in &self.world.state.harbors {
			if terrain.torus_bounds_check(left_top, right_bottom, harbor.loc) {
				let remapped = terrain.torus_remap(left_top, harbor.loc);

				let harbor_scale = logic::glm::vec1(
					2. * logic::HARBOR_SIZE * pixel_per_meter
						/ self.images.building_batches.harbor.params().width as f32,
				)
				.xx();
				let harbor_pos = remapped.0 - logic::glm::vec1(2. * logic::HARBOR_SIZE).xx() * 0.5;
				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(harbor_pos)))
					.scale(harbor_scale);

				self.images.building_batches.harbor.add_frame(
					0.0,
					f64::from(harbor.orientation),
					0.0,
					param,
				);
			}
		}

		// Draw and clear sprite batches
		// This defines the draw order.
		draw_and_clear(
			ctx,
			quad_ctx,
			[].into_iter()
				.chain([
					&mut self.images.terrain_batches.deep,
					&mut self.images.terrain_batches.shallow,
				])
				.chain(
					self.images
						.resource_batches
						.fishes
						.iter_mut()
						.map(DerefMut::deref_mut),
				)
				.chain([
					&mut self.images.terrain_batches.water_anim,
					&mut self.images.terrain_batches.water_anim_2,
					&mut self.images.terrain_batches.beach,
					&mut self.images.terrain_batches.grass,
				]),
		)?;

		// Draw the tile background
		for (tc, tile) in terrain.iter() {
			if terrain.torus_bounds_check(left_top, right_bottom, tc.to_location()) {
				let remapped = terrain.torus_remap(left_top, tc.to_location());

				let screen_size = logic::TILE_SIZE as f32 * pixel_per_meter;
				let scale = screen_size / tile_image_size;
				let loc = remapped.0 - half_tile;
				let dest = self.location_to_screen_coords(ctx, Location(loc));

				let c = 1.;
				let param = DrawParam::new()
					.dest(dest)
					.scale(logic::glm::vec2(scale, scale))
					.color(Color::new(c, c, c, 1.));

				let class = tile.classify();

				// Edges

				let easter = terrain.get(terrain.east_of(tc)).classify();
				if class < easter {
					self.images.terrain_batches.tile_sprite(easter).add(param);
					let param_rot = param.clone();
					self.images
						.terrain_batches
						.tile_mask_s1(easter)
						.add(param_rot);
				}
				let southern = terrain.get(terrain.south_of(tc)).classify();
				if class < southern {
					self.images.terrain_batches.tile_sprite(southern).add(param);
					let param_rot = param
						.clone()
						.rotation(std::f32::consts::PI / 2.)
						.dest(dest + logic::glm::vec2(screen_size, 0.));
					self.images
						.terrain_batches
						.tile_mask_s1(southern)
						.add(param_rot);
				}
				let western = terrain.get(terrain.west_of(tc)).classify();
				if class < western {
					self.images.terrain_batches.tile_sprite(western).add(param);
					let param_rot = param
						.clone()
						.rotation(std::f32::consts::PI)
						.dest(dest + logic::glm::vec2(screen_size, screen_size));
					self.images
						.terrain_batches
						.tile_mask_s1(western)
						.add(param_rot);
				}
				let norther = terrain.get(terrain.north_of(tc)).classify();
				if class < norther {
					self.images.terrain_batches.tile_sprite(norther).add(param);
					let param_rot = param
						.clone()
						.rotation(-std::f32::consts::PI / 2.)
						.dest(dest + logic::glm::vec2(0., screen_size));
					self.images
						.terrain_batches
						.tile_mask_s1(norther)
						.add(param_rot);
				}

				// Corners

				let ne = terrain.get(terrain.north_of(terrain.east_of(tc))).classify();
				if class < ne {
					self.images.terrain_batches.tile_sprite(ne).add(param);
					let param_rot = param.clone();
					self.images
						.terrain_batches
						.tile_mask_c1(ne)
						.add(param_rot);
				}
				let se = terrain.get(terrain.south_of(terrain.east_of(tc))).classify();
				if class < se {
					self.images.terrain_batches.tile_sprite(se).add(param);
					let param_rot = param
						.clone()
						.rotation(std::f32::consts::PI / 2.)
						.dest(dest + logic::glm::vec2(screen_size, 0.));
					self.images
						.terrain_batches
						.tile_mask_c1(se)
						.add(param_rot);
				}
				let sw = terrain.get(terrain.south_of(terrain.west_of(tc))).classify();
				if class < sw {
					self.images.terrain_batches.tile_sprite(sw).add(param);
					let param_rot = param
						.clone()
						.rotation(std::f32::consts::PI)
						.dest(dest + logic::glm::vec2(screen_size, screen_size));
					self.images
						.terrain_batches
						.tile_mask_c1(sw)
						.add(param_rot);
				}
				let nw = terrain.get(terrain.north_of(terrain.west_of(tc))).classify();
				if class < nw {
					self.images.terrain_batches.tile_sprite(nw).add(param);
					let param_rot = param
						.clone()
						.rotation(-std::f32::consts::PI / 2.)
						.dest(dest + logic::glm::vec2(0., screen_size));
					self.images
						.terrain_batches
						.tile_mask_c1(nw)
						.add(param_rot);
				}
			}
		}

		// The Mask itself is draw multiplicative
		self.terrain_transition_mask_canvas.set_blend_mode(Some(BlendMode::Multiply));

		let mask_canvas = &self.terrain_transition_mask_canvas;
		let trans_canvas = &self.terrain_transition_canvas;

		fn draw_mask_n_tiles(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,mask_canvas:&Canvas,trans_canvas:&Canvas,mask:Vec<&mut SpriteBatch>,tile:&mut SpriteBatch) -> GameResult {
			// The mask canvas, needs to be cleared with white
		graphics::set_canvas(ctx, Some(mask_canvas));
		graphics::clear(ctx, quad_ctx, [1.0, 1.0, 1.0, 0.0].into());

		// Drawing the mask
		draw_and_clear(ctx, quad_ctx, mask)?;

		// The Tile canvas
		graphics::set_canvas(ctx, Some(&trans_canvas));
		graphics::clear(ctx, quad_ctx, [0.0, 0.0, 0.0, 0.0].into());

		// Drawing the tile
		draw_and_clear(ctx, quad_ctx, [tile])?;

		// And multiplying the mask on top
		graphics::draw(
			ctx,
			quad_ctx,
			mask_canvas,
			(Point2::new(0., 0.),),
		)?;

		// Switch back to the screen
		graphics::set_canvas(ctx, None);

		// Draw the transition tiles
		graphics::draw(
			ctx,
			quad_ctx,
			trans_canvas,
			(Point2::new(0., 0.),),
		)
		}

		let (tile,mask) = self.images.terrain_batches.shallow_batches();
		draw_mask_n_tiles(ctx,quad_ctx,mask_canvas,trans_canvas, mask, tile)?;

		let (tile2,mask2) = self.images.terrain_batches.beach_batches();
		draw_mask_n_tiles(ctx,quad_ctx,mask_canvas,trans_canvas, mask2, tile2)?;

		let (tile3,mask3) = self.images.terrain_batches.grass_batches();
		draw_mask_n_tiles(ctx,quad_ctx,mask_canvas,trans_canvas, mask3, tile3)?;


		// Draw and clear sprite batches
		// This defines the draw order.
		draw_and_clear(
			ctx,
			quad_ctx,
			[].into_iter()
				.chain([self.images.building_batches.harbor.deref_mut()])
				.chain(
					self.images
						.ship_batches
						.basic
						.body
						.values_mut()
						.map(|s| s.deref_mut()),
				)
				.chain(
					self.images
						.ship_batches
						.basic
						.sail
						.values_mut()
						.flat_map(|s| s.iter_mut().map(DerefMut::deref_mut)),
				),
		)?;

		// Draw some debugging stuff
		self.draw_debugging(ctx, quad_ctx)?;

		self.draw_ui(glob, ctx, quad_ctx)?;

		// From the text example
		// Source: https://github.com/ggez/good-web-game/blob/master/examples/text.rs
		let fps = timer::fps(ctx);
		let fps_display = Text::new(format!("FPS: {:.1}", fps));
		// When drawing through these calls, `DrawParam` will work as they are documented.
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&fps_display,
			(Point2::new(100.0, 0.0), Color::WHITE),
			Color::BLACK,
		)?;

		// Current input state
		let input_text = Text::new(format!("Input: {:?}", self.input));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 20.0), Color::WHITE),
			Color::BLACK,
		)?;

		// Current Wind
		let input_text = Text::new(format!(
			"Wind: {:.2} bf, {:.0}°",
			self.world.state.wind.magnitude(),
			self.world.state.wind.angle().to_degrees(),
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 40.0), Color::WHITE),
			Color::BLACK,
		)?;
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&Text::new("→"),
			(
				Point2::new(70.0, 40.0),
				self.world.state.wind.angle(),
				Color::WHITE,
			),
			Color::BLACK,
		)?;

		// Current Ship states
		let input_text = Text::new(format!(
			"Ship: {:.2} m/s, fish {:.0} kg",
			self.world.state.player.vehicle.ground_speed(),
			self.world.state.player.vehicle.fish.0
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 60.0), Color::WHITE),
			Color::BLACK,
		)?;

		// Current Ship states
		let input_text = Text::new(format!(
			"Ori: {:.2}, List: {:.0}°",
			self.world
				.state
				.player
				.vehicle
				.heading
				.rem_euclid(std::f32::consts::TAU),
			self.world.state.player.vehicle.angle_of_list.to_degrees()
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 80.0), Color::WHITE),
			Color::BLACK,
		)?;

		// Show players money
		let money = Text::new(format!("Money: {:} ℓ", self.world.state.player.money));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&money,
			(Point2::new(0.0, 0.0), Color::WHITE),
			Color::BLACK,
		)?;

		// Show the current amount of fish in the ship
		let fish = Text::new(format!(
			"Fish: {:} kg",
			self.world.state.player.vehicle.fish.0
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&fish,
			(Point2::new(0.0, 20.0), Color::WHITE),
			Color::BLACK,
		)?;

		if let Some(t) = self.world.state.get_trading() {
			if t.has_player_valid_speed() {
				// Trading is possible

				let mut text = Text::new("Welcome to the harbor");
				text.set_font(Default::default(), PxScale::from(32.));
				let h = text.dimensions(ctx).h;
				graphics::draw(
					ctx,
					quad_ctx,
					&text,
					(
						Point2::new(0.0, screen_coords.h / 2. - h - 5.),
						Color::BLACK,
					),
				)?;

				let mut text = Text::new("Your opportunities:");
				text.set_font(Default::default(), PxScale::from(28.));
				graphics::draw(
					ctx,
					quad_ctx,
					&text,
					(Point2::new(0.0, screen_coords.h / 2.), Color::BLACK),
				)?;
				let h = text.dimensions(ctx).h;

				let selling = {
					if t.players_fish_amount() > 0 {
						format!("S: sell fish at {} ℓ/kg", t.get_price_for_fish())
					} else {
						"You need more fish!".to_string()
					}
				};
				let up_sail = {
					if let Some(price) = t.get_price_for_sail_upgrade() {
						format!("U: upgrade sail for {} ℓ", price)
					} else {
						"You have the best sail already".to_string()
					}
				};
				let up_hull = {
					if let Some(price) = t.get_price_of_hull_upgrade() {
						format!("H: buy a new ship hull for {} ℓ", price)
					} else {
						"You have the best ship already".to_string()
					}
				};

				let mut text = Text::new(format!("{selling}\n{up_sail}\n{up_hull}",));
				text.set_font(Default::default(), PxScale::from(20.));
				graphics::draw(
					ctx,
					quad_ctx,
					&text,
					(Point2::new(0.0, screen_coords.h / 2. + h), Color::BLACK),
				)?;
			} else {
				// Player is too fast for trading

				let mut text = Text::new("You are too fast to interact with the harbor");
				text.set_font(Default::default(), PxScale::from(32.));
				graphics::draw(
					ctx,
					quad_ctx,
					&text,
					(Point2::new(0.0, screen_coords.h / 2.), Color::BLACK),
				)?;
			}
		}

		// Print version info
		draw_version(ctx, quad_ctx)?;

		gwg::graphics::present(ctx, quad_ctx)?;

		Ok(())
	}

	fn key_down_event(
		&mut self,
		_glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		keycode: gwg::miniquad::KeyCode,
	) {
		if keycode == KeyCode::Escape {
			gwg::event::quit(ctx);
		}

		if keycode == KeyCode::Enter || keycode == KeyCode::KpEnter {
			self.audios.sound.play(ctx).unwrap()
		}

		if keycode == KeyCode::A {
			self.audios.sound.play(ctx).unwrap();
			self.audios.music_0.stop(ctx).unwrap();
			self.audios.music_0.play(ctx).unwrap();
		}

		if keycode == KeyCode::KpAdd {
			self.zoom_factor_exp = self.zoom_factor_exp.saturating_add(1);
		}
		if keycode == KeyCode::KpSubtract {
			self.zoom_factor_exp = self.zoom_factor_exp.saturating_sub(1);
		}
		if keycode == KeyCode::Kp0 {
			self.zoom_factor_exp = DEFAULT_ZOOM_LEVEL;
		}
		if keycode == KeyCode::U {
			// Check whether the player is at a harbor
			if let Some(mut t) = self.world.state.get_trading() {
				if t.has_player_valid_speed() {
					let n = t.upgrade_sail();
					match n {
						Ok(()) => {
							// success
							if self.audios.sound_enabled {
								self.audios.upgrade_sound.play(ctx).unwrap();
							}
						},
						Err(e) => {
							// Failed
							println!("Failed to upgrade sail: {e}");
							if self.audios.sound_enabled {
								self.audios.fail_sound.play(ctx).unwrap();
							}
						},
					}
				}
			}
		}
		if keycode == KeyCode::H {
			// Check whether the player is at a harbor
			if let Some(mut t) = self.world.state.get_trading() {
				if t.has_player_valid_speed() {
					let n = t.upgrade_hull();
					match n {
						Ok(()) => {
							// success
							if self.audios.sound_enabled {
								self.audios.upgrade_sound.play(ctx).unwrap();
							}
						},
						Err(e) => {
							// Failed
							println!("Failed to upgrade sail: {e}");
							if self.audios.sound_enabled {
								self.audios.fail_sound.play(ctx).unwrap();
							}
						},
					}
				}
			}
		}

		// Reefing input
		if keycode == KeyCode::Up {
			self.input.reefing = self.input.reefing.increase();

			// Limit reefing
			let max_reefing = self.world.state.player.vehicle.sail.kind.max_reefing();
			if self.input.reefing > max_reefing {
				self.input.reefing = max_reefing;
			}
		} else if keycode == KeyCode::Down {
			self.input.reefing = self.input.reefing.decrease();
		}

		if keycode == KeyCode::F11 {
			self.full_screen = !self.full_screen;
			println!("{}", self.full_screen);
			good_web_game::graphics::set_fullscreen(quad_ctx, self.full_screen);
		}
	}

	/*
	TODO: what to do about that?
	fn text_input_event(
		&mut self,
		_ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
		character: char,
	) {
		self.input_text.push(character);
	}
	*/

	fn resize_event(
		&mut self,
		_glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::GraphicsContext,
		w: f32,
		h: f32,
	) {
		let coordinates = graphics::Rect::new(0., 0., w, h);

		graphics::set_screen_coordinates(ctx, coordinates).expect("Can't resize the window");
		self.terrain_transition_canvas = Canvas::with_window_size(ctx, quad_ctx).unwrap();
		self.terrain_transition_mask_canvas = Canvas::with_window_size(ctx, quad_ctx).unwrap();
	}
}

impl Game {
	fn draw_ui(
		&mut self,
		_glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		let screen_coords = gwg::graphics::screen_coordinates(ctx);

		let normed_wind_speed = self.world.state.wind.magnitude() / logic::MAX_WIND_SPEED;
		let n_colors = self.images.ui.wind_speed_colors.len();
		let color_idx_f32 = n_colors.saturating_sub(1) as f32 * normed_wind_speed;
		let color_idx1 = color_idx_f32 as usize;
		let color_idx2 = (color_idx1 + 1).min(n_colors.saturating_sub(1));
		let mix_factor = color_idx_f32.fract();

		let color1 = &self.images.ui.wind_speed_colors[color_idx1];
		let color2 = &self.images.ui.wind_speed_colors[color_idx2];

		let color = color1.mix(color2, mix_factor);

		let mut wind_text = Text::new(format!(
			"{:.2} bf, {:.0}°",
			self.world.state.wind.magnitude(),
			self.world.state.wind.angle().to_degrees(),
		));
		wind_text.set_font(Default::default(), PxScale::from(20.));

		let p = DrawParam::new()
			.dest(Point2::new(
				screen_coords.w - 128.0,
				screen_coords.h - 128.0 - wind_text.height(ctx),
			))
			.offset(Point2::new(0.5, 0.5))
			.color(color)
			.scale(logic::glm::vec1(normed_wind_speed).xx())
			.rotation(self.world.state.wind.angle() + std::f32::consts::FRAC_PI_2);
		gwg::graphics::draw(ctx, quad_ctx, &self.images.ui.wind_direction_indicator, p)?;

		let p = DrawParam::new()
			.dest(Point2::new(
				screen_coords.w - 128.0 - wind_text.width(ctx) * 0.5,
				screen_coords.h - wind_text.height(ctx),
			))
			.color(color);
		self.draw_text_with_halo(ctx, quad_ctx, &wind_text, p, Color::BLACK)?;

		Ok(())
	}
}
