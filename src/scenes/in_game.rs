use std::ops::DerefMut;
use std::path::Path;

use cfg_if::cfg_if;
use enum_map::enum_map;
use good_web_game as gwg;
use gwg::goodies::scene::Scene;
use gwg::goodies::scene::SceneSwitch;
use gwg::graphics;
use gwg::graphics::draw;
use gwg::graphics::spritebatch::SpriteBatch;
use gwg::graphics::BlendMode;
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
use gwg::miniquad::KeyCode;
use gwg::timer;
use gwg::GameResult;
use logic::generator::Generator;
use logic::generator::PerlinNoise;
use logic::generator::Setting;
use logic::glm::vec1;
use logic::glm::vec2;
use logic::glm::Vec2;
use logic::resource::ResourcePackContent;
use logic::state::Event;
use logic::state::SailKind;
use logic::terrain::TileCoord;
use logic::units::BiPolarFraction;
use logic::units::Distance;
use logic::units::Elevation;
use logic::units::Location;
use logic::units::TileType;
use logic::Input;
use logic::World;
use logic::TICKS_PER_SECOND;
use logic::TILE_SIZE;
use nalgebra::Point2;
use rand::seq::SliceRandom;
use rand::Rng;
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
use crate::math::Line;

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
const DEFAULT_ZOOM_LEVEL: i32 = -1;

/// Probability of catching a compliment when catching a fish, in percent
const COMPLIMENT_PROBABILITY: f64 = 0.01;

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


const COMPLIMENTS: &[&str] = &[
	"You're the best!",
	"You're so talented!",
	"You're one of a kind!",
	"You're a living legend!",
];

// #[derive(Debug)] `audio::Source` dose not implement Debug!
pub struct Game {
	/// The drawables
	images: Images,

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

	/// True in the very first frame
	init: bool,

	available_compliments: Vec<&'static str>,
	fished_compliments: Vec<&'static str>,
}

impl Game {
	pub(super) fn new(
		glob: &mut GlobalState,
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
		let music_enabled = !opts.muted;

		println!(
			"{:.3} [game] loading sounds...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		glob.audios
			.as_mut()
			.unwrap()
			.enable_sound(ctx, sound_enabled)?;
		glob.audios
			.as_mut()
			.unwrap()
			.enable_music(ctx, music_enabled)?;

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

			shallow_solid: SpriteBatch::new(Image::solid(ctx, quad_ctx, 1, Color::WHITE)?),
			shallow_c1: image_batch(ctx, quad_ctx, "img/mask_shallow_c1.png")?,
			shallow_s1: image_batch(ctx, quad_ctx, "img/mask_shallow_s1.png")?,
			shallow_s2: image_batch(ctx, quad_ctx, "img/mask_shallow_s2.png")?,
			shallow_s3: image_batch(ctx, quad_ctx, "img/mask_shallow_s3.png")?,
			shallow_s4: image_batch(ctx, quad_ctx, "img/mask_shallow_s4.png")?,

			beach_solid: SpriteBatch::new(Image::solid(ctx, quad_ctx, 1, Color::WHITE)?),
			beach_c1: image_batch(ctx, quad_ctx, "img/mask_sand_c1.png")?,
			beach_s1: image_batch(ctx, quad_ctx, "img/mask_sand_s1.png")?,
			beach_s2: image_batch(ctx, quad_ctx, "img/mask_sand_s2.png")?,
			beach_s3: image_batch(ctx, quad_ctx, "img/mask_sand_s3.png")?,
			beach_s4: image_batch(ctx, quad_ctx, "img/mask_sand_s4.png")?,

			grass_solid: SpriteBatch::new(Image::solid(ctx, quad_ctx, 1, Color::WHITE)?),
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
		let mut map_to_ass =
			|names: Vec<&str>| {
				Vec::from_iter(names.into_iter().map(|name| {
					AssetBatch::from_config(ctx, quad_ctx, &render_config, name).unwrap()
				}))
			};
		let resource_batches = ResourceBatches {
			fishes: map_to_ass(vec![
				"fish-00", "fish-01", "fish-02", "fish-03", "fish-04", "fish-05", "fish-06",
				"fish-07",
			]),
			starfishes: map_to_ass(vec![
				"starfish-00",
				"starfish-01",
				"starfish-02",
				"starfish-03",
				"starfish-04",
			]),
			shoe: map_to_ass(vec!["shoe-01", "shoe-00"]),
			grass: map_to_ass(vec!["grass-00", "grass-01"]),
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
			wind_speed_colors: vec![Color::BLUE, Color::WHITE, Color::GREEN],
			harbor_indicator: Image::new(ctx, quad_ctx, Path::new("img/moneybag_col.png")).unwrap(),
			money_icon: Image::new(ctx, quad_ctx, Path::new("img/money_icon.png")).unwrap(),
			fishy_icon: Image::new(ctx, quad_ctx, Path::new("img/fish-icon.png")).unwrap(),
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
		let noise = PerlinNoise; // logic::generator::WhiteNoise
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
		// Find a starting position for the player
		let start_point = world.state.harbors[0].loc;
		let mut dist = 2_i32;
		'find_pos: loop {
			let forward = ((-dist)..=dist).map(|n| (n, 1));
			let backward = ((1 - dist)..=(dist - 1)).map(|n| (n, -1));
			let mut offsets = Vec::from_iter(forward.chain(backward));
			offsets.shuffle(&mut rng);
			for (x, s) in offsets {
				let y = (dist - x.abs()) * s;

				let diff = vec2(x as f32, y as f32) * logic::HARBOR_SIZE;
				let candidate = start_point + Distance(diff);
				let candidate = world.init.terrain.map_loc_on_torus(candidate);

				if world
					.init
					.terrain
					.get(candidate.try_into().unwrap())
					.is_passable()
				{
					world.state.player.vehicle.pos = candidate;
					// Orient orthogonal to the distance to the harbor
					world.state.player.vehicle.heading = f32::atan2(x as f32, -y as f32);
					break 'find_pos;
				}
			}

			dist += 1;
		}
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
			terrain_transition_canvas,
			terrain_transition_mask_canvas,
			full_screen: !opts.windowed,
			world,
			input: Input::default(),
			zoom_factor_exp: DEFAULT_ZOOM_LEVEL,
			water_wave_offset: Default::default(),
			water_wave_2_offset: Default::default(),
			init: true,
			available_compliments: COMPLIMENTS.to_owned(),
			fished_compliments: Vec::new(),
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

			// Line to the home harbor
			let p_pos = self.world.state.player.vehicle.pos;
			let dist = self
				.world
				.init
				.terrain
				.torus_distance(p_pos, self.world.state.harbors[0].loc);
			let dist = Distance(dist.0 * 0.5);
			let mesh = MeshBuilder::new()
				.line(
					&[
						self.location_to_screen_coords(ctx, p_pos),
						self.location_to_screen_coords(ctx, p_pos + dist),
					],
					1.,
					Color::RED,
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
		glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
	) -> SceneSwitch<GlobalState> {
		use gwg::input::keyboard::is_key_pressed;

		let audios = glob.audios.as_mut().unwrap();

		let mut rng = wyhash::WyRng::seed_from_u64((gwg::timer::time() * 1000.) as u64);

		let mut did_trade_successful = false;
		let mut did_trade_fail = false;

		let mut collision_harbor_in_this_frame = false;
		let mut collision_beach_in_this_frame = false;

		let mut collision_harbor_in_this_frame_st = 0.0_f32;
		let mut collision_beach_in_this_frame_st = 0.0_f32;

		let mut tickies = 0;
		while gwg::timer::check_update_time(ctx, TICKS_PER_SECOND.into()) {
			tickies += 1;
			if self.init && tickies > 1 {
				// Just ignore additional frames
				continue;
			}
			if tickies > 10 {
				// Just ignore additional frames
				continue;
			}

			// Rudder input
			let mut rudder = 0.0;
			if is_key_pressed(ctx, KeyCode::Left) || is_key_pressed(ctx, KeyCode::A) {
				rudder -= 1.0;
			}
			if is_key_pressed(ctx, KeyCode::Right) || is_key_pressed(ctx, KeyCode::D) {
				rudder += 1.0;
			}

			self.input.rudder = BiPolarFraction::from_f32(rudder).unwrap();
			let events = self.world.state.update(&self.world.init, &self.input);

			// Play event sounds
			if audios.sound_enabled {
				for ev in events {
					match ev {
						Event::Fishy => {
							let fishies = [
								&audios.sound_fishy_1,
								&audios.sound_fishy_2,
								&audios.sound_fishy_3,
							];
							let sound = fishies.choose(&mut rng).unwrap();

							if !self.available_compliments.is_empty()
								&& rng.gen_bool(COMPLIMENT_PROBABILITY)
							{
								let compliment_index =
									rng.gen_range(0..self.available_compliments.len());
								let compliment =
									self.available_compliments.swap_remove(compliment_index);
								self.fished_compliments.push(compliment);
							}

							sound.play(ctx).unwrap();
						},
						Event::Shoe => {
							let shoe = [&audios.sound_shoe];
							let sound = shoe.choose(&mut rng).unwrap();

							sound.play(ctx).unwrap();
						},
						Event::Starfish => {
							let star = [&audios.sound_blub];
							let sound = star.choose(&mut rng).unwrap();

							sound.play(ctx).unwrap();
						},
						Event::Grass => {
							let grass = [&audios.sound_grass];
							let sound = grass.choose(&mut rng).unwrap();

							sound.play(ctx).unwrap();
						},
						Event::HarborCollision(s) => {
							collision_harbor_in_this_frame = true;
							collision_harbor_in_this_frame_st =
								collision_harbor_in_this_frame_st.max(s);
						},
						Event::TileCollision(s) => {
							collision_beach_in_this_frame = true;
							collision_beach_in_this_frame_st =
								collision_beach_in_this_frame_st.max(s);
						},
					}
				}
			}

			// Selling (fixed with logic ticks, so it is independent from the frame rate)
			if let Some(mut trade) = self.world.state.get_trading(&self.world.init) {
				if is_key_pressed(ctx, KeyCode::E) {
					let res = trade.sell_fish(10);
					if let Some(proceeds) = res {
						if proceeds > 0 {
							did_trade_successful = true;
						} else {
							did_trade_fail = true;
						}
					}
				}
			}
		}
		// Play collision event sounds
		if audios.sound_enabled {
			if collision_harbor_in_this_frame && !audios.collision_harbor_in_this_frame {
				let mut harbor = [&mut audios.collision_harbor];
				let sound = harbor.choose_mut(&mut rng).unwrap();

				sound
					.set_volume(ctx, collision_harbor_in_this_frame_st.clamp(0.0, 2.0))
					.unwrap();
				sound.play(ctx).unwrap();
			}
			audios.collision_harbor_in_this_frame = collision_harbor_in_this_frame;
			if collision_beach_in_this_frame && !audios.collision_beach_in_this_frame {
				let mut beach = [&mut audios.collision_beach];
				let sound = beach.choose_mut(&mut rng).unwrap();

				sound
					.set_volume(ctx, collision_beach_in_this_frame_st.clamp(0.0, 2.0))
					.unwrap();
				sound.play(ctx).unwrap();
			}
			audios.collision_beach_in_this_frame = collision_beach_in_this_frame;
		}

		audios
			.sell_sound
			.set_volume(ctx, did_trade_successful as u8 as f32)
			.unwrap();

		if audios.sound_enabled && did_trade_fail && !did_trade_successful {
			audios.fail_sound.play(ctx).unwrap();
		}

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
		audios
			.water_sound_1
			.set_volume(ctx, normalized_rel_water_speed * 2.)
			.unwrap();

		self.init = false;

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
				.min(terrain.map_size() - 5. * logic::TILE_SIZE as f32);
			let scm_y = (screen_coords.h / pixel_per_meter)
				.min(terrain.map_size() - 5. * logic::TILE_SIZE as f32);
			let dst = Distance::new(scm_x * 0.5, scm_y * 0.5);

			let lt = player_pos - dst - Distance(full_tile * 2.);
			let rb = player_pos + dst + Distance(full_tile * 2.);

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

		let ship_pos = self.world.state.player.vehicle.pos.0
			- logic::glm::vec1(1.22 * 2.5 * logic::VEHICLE_SIZE).xx() * 0.5;
		let ship_screen_loc = self.location_to_screen_coords(ctx, Location(ship_pos));

		let body = &mut self.images.ship_batches.basic.body[self.world.state.player.vehicle.hull];

		// Draw the player ship
		let ship_scale = logic::glm::vec1(
			1.22 * 2.5 * logic::VEHICLE_SIZE * pixel_per_meter / body.params().width as f32,
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
			1.22 * 2.5 * logic::VEHICLE_SIZE * pixel_per_meter / sail_ass.params().width as f32,
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
					remapped.0 - logic::glm::vec1(1.22 * logic::RESOURCE_PACK_FISH_SIZE).xx() * 0.5;
				let dest = self.location_to_screen_coords(ctx, Location(resource_pos));

				let batch = match resource.content {
					ResourcePackContent::Fish0 => &mut self.images.resource_batches.fishes[0],
					ResourcePackContent::Fish1 => &mut self.images.resource_batches.fishes[1],
					ResourcePackContent::Fish2 => &mut self.images.resource_batches.fishes[2],
					ResourcePackContent::Fish3 => &mut self.images.resource_batches.fishes[3],
					ResourcePackContent::Fish4 => &mut self.images.resource_batches.fishes[4],
					ResourcePackContent::Fish5 => &mut self.images.resource_batches.fishes[5],
					ResourcePackContent::Fish6 => &mut self.images.resource_batches.fishes[6],
					ResourcePackContent::Fish7 => &mut self.images.resource_batches.fishes[7],
					ResourcePackContent::Shoe0 => &mut self.images.resource_batches.shoe[0],
					ResourcePackContent::Shoe1 => &mut self.images.resource_batches.shoe[1],
					ResourcePackContent::Starfish0 => {
						&mut self.images.resource_batches.starfishes[0]
					},
					ResourcePackContent::Starfish1 => {
						&mut self.images.resource_batches.starfishes[1]
					},
					ResourcePackContent::Starfish2 => {
						&mut self.images.resource_batches.starfishes[2]
					},
					ResourcePackContent::Starfish3 => {
						&mut self.images.resource_batches.starfishes[3]
					},
					ResourcePackContent::Starfish4 => {
						&mut self.images.resource_batches.starfishes[4]
					},
					ResourcePackContent::Grass0 => &mut self.images.resource_batches.grass[0],
					ResourcePackContent::Grass1 => &mut self.images.resource_batches.grass[1],
				};

				let resource_scale = logic::glm::vec1(
					1.22 * logic::RESOURCE_PACK_FISH_SIZE * pixel_per_meter
						/ batch.params().width as f32,
				)
				.xx();

				let max_depth = Elevation::DEEPEST.0;
				let depth = (f32::from(resource.elevation.0 - max_depth) / f32::from(-max_depth))
					.clamp(0., 1.);
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
					1.22 * 2. * logic::HARBOR_SIZE * pixel_per_meter
						/ self.images.building_batches.harbor.params().width as f32,
				)
				.xx();
				let harbor_pos =
					remapped.0 - logic::glm::vec1(1.22 * 2. * logic::HARBOR_SIZE).xx() * 0.5;
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

		// Draw the tile background
		for (tc, tile) in terrain.iter() {
			if terrain.torus_bounds_check(left_top, right_bottom, tc.to_location()) {
				let remapped = terrain.torus_remap(left_top, tc.to_location());

				let screen_size = logic::TILE_SIZE as f32 * pixel_per_meter;
				let scale = screen_size / tile_image_size;
				let loc = remapped.0 - half_tile;
				let dest = self.location_to_screen_coords(ctx, Location(loc));

				// Depth shading

				/*
				let rel = match tile.classify() {
					TileType::DeepWater => tile.relative_height(),
					TileType::ShallowWater => tile.relative_height() * 0.5 + 0.5,
					TileType::Beach => 1.0,
					TileType::Grass => 1.0,
				};
				*/
				let rel = 1.0_f32;
				let c = 0.5 + 0.5 * rel.clamp(0., 1.);

				let param = DrawParam::new()
					.dest(dest)
					.scale(logic::glm::vec2(scale, scale))
					.color(Color::new(c, c, c, 1.));

				let class = tile.classify();

				// Main tile

				self.images.terrain_batches.tile_sprite(class).add(param);
				if class != TileType::DeepWater {
					let solid_mask_param = param.scale(logic::glm::vec2(screen_size, screen_size));
					self.images
						.terrain_batches
						.tile_mask_solid(class)
						.add(solid_mask_param);
				}

				// Sides

				let eastern = terrain.get(terrain.east_of(tc)).classify();
				let southern = terrain.get(terrain.south_of(tc)).classify();
				let western = terrain.get(terrain.west_of(tc)).classify();
				let northern = terrain.get(terrain.north_of(tc)).classify();

				let ne_eq = northern == eastern;
				let nw_eq = northern == western;
				let se_eq = southern == eastern;
				let sw_eq = southern == western;

				if class < eastern && ne_eq && nw_eq && se_eq && sw_eq {
					// Full four sides

					// The base tile (to be made into a transition via mask)
					self.images.terrain_batches.tile_sprite(eastern).add(param);

					// TODO: how about randomizing the orientation?
					self.images.terrain_batches.tile_mask_s4(eastern).add(param);
				} else {
					if class < eastern && !ne_eq {
						// Other class
						let other_class = eastern;

						// The base tile (to be made into a transition via mask)
						self.images
							.terrain_batches
							.tile_sprite(other_class)
							.add(param);

						// The rotation of the mask
						let param_rot = param;

						// Determine the mask to be used
						if !se_eq {
							// Single edge, just a straight edge
							self.images
								.terrain_batches
								.tile_mask_s1(other_class)
								.add(param_rot);
						} else if !sw_eq {
							// Double edge, aka an inner corner
							self.images
								.terrain_batches
								.tile_mask_s2(other_class)
								.add(param_rot);
						} else {
							// Since NE is not equal, NW must not as well
							debug_assert!(!nw_eq);

							// Triple edge, aka a bay
							self.images
								.terrain_batches
								.tile_mask_s3(other_class)
								.add(param_rot);
						}
					}
					if class < southern && !se_eq {
						// Other class
						let other_class = southern;

						// The base tile (to be made into a transition via mask)
						self.images
							.terrain_batches
							.tile_sprite(other_class)
							.add(param);

						// The rotation of the mask
						let param_rot = param
							.rotation(std::f32::consts::PI / 2.)
							.dest(dest + logic::glm::vec2(screen_size, 0.));

						// Determine the mask to be used
						if !sw_eq {
							// Single edge, just a straight edge
							self.images
								.terrain_batches
								.tile_mask_s1(other_class)
								.add(param_rot);
						} else if !nw_eq {
							// Double edge, aka an inner corner
							self.images
								.terrain_batches
								.tile_mask_s2(other_class)
								.add(param_rot);
						} else {
							// Since NE is not equal, NW must not as well
							debug_assert!(!ne_eq);

							// Triple edge, aka a bay
							self.images
								.terrain_batches
								.tile_mask_s3(other_class)
								.add(param_rot);
						}
					}
					if class < western && !sw_eq {
						// Other class
						let other_class = western;

						// The base tile (to be made into a transition via mask)
						self.images
							.terrain_batches
							.tile_sprite(other_class)
							.add(param);

						// The rotation of the mask
						let param_rot = param
							.rotation(std::f32::consts::PI)
							.dest(dest + logic::glm::vec2(screen_size, screen_size));

						// Determine the mask to be used
						if !nw_eq {
							// Single edge, just a straight edge
							self.images
								.terrain_batches
								.tile_mask_s1(other_class)
								.add(param_rot);
						} else if !ne_eq {
							// Double edge, aka an inner corner
							self.images
								.terrain_batches
								.tile_mask_s2(other_class)
								.add(param_rot);
						} else {
							// Since NE is not equal, NW must not as well
							debug_assert!(!se_eq);

							// Triple edge, aka a bay
							self.images
								.terrain_batches
								.tile_mask_s3(other_class)
								.add(param_rot);
						}
					}
					if class < northern && !nw_eq {
						// Other class
						let other_class = northern;

						// The base tile (to be made into a transition via mask)
						self.images
							.terrain_batches
							.tile_sprite(other_class)
							.add(param);

						// The rotation of the mask
						let param_rot = param
							.rotation(-std::f32::consts::PI / 2.)
							.dest(dest + logic::glm::vec2(0., screen_size));

						// Determine the mask to be used
						if !ne_eq {
							// Single edge, just a straight edge
							self.images
								.terrain_batches
								.tile_mask_s1(other_class)
								.add(param_rot);
						} else if !se_eq {
							// Double edge, aka an inner corner
							self.images
								.terrain_batches
								.tile_mask_s2(other_class)
								.add(param_rot);
						} else {
							// Since NE is not equal, NW must not as well
							debug_assert!(!sw_eq);

							// Triple edge, aka a bay
							self.images
								.terrain_batches
								.tile_mask_s3(other_class)
								.add(param_rot);
						}
					}
				}

				// Corners

				let north_east = terrain
					.get(terrain.north_of(terrain.east_of(tc)))
					.classify();
				if class < north_east && (north_east != northern && north_east != eastern) {
					self.images
						.terrain_batches
						.tile_sprite(north_east)
						.add(param);
					let param_rot = param;
					self.images
						.terrain_batches
						.tile_mask_c1(north_east)
						.add(param_rot);
				}
				let south_east = terrain
					.get(terrain.south_of(terrain.east_of(tc)))
					.classify();
				if class < south_east && (south_east != southern && south_east != eastern) {
					self.images
						.terrain_batches
						.tile_sprite(south_east)
						.add(param);
					let param_rot = param
						.rotation(std::f32::consts::PI / 2.)
						.dest(dest + logic::glm::vec2(screen_size, 0.));
					self.images
						.terrain_batches
						.tile_mask_c1(south_east)
						.add(param_rot);
				}
				let south_west = terrain
					.get(terrain.south_of(terrain.west_of(tc)))
					.classify();
				if class < south_west && (south_west != southern && south_west != western) {
					self.images
						.terrain_batches
						.tile_sprite(south_west)
						.add(param);
					let param_rot = param
						.rotation(std::f32::consts::PI)
						.dest(dest + logic::glm::vec2(screen_size, screen_size));
					self.images
						.terrain_batches
						.tile_mask_c1(south_west)
						.add(param_rot);
				}
				let north_west = terrain
					.get(terrain.north_of(terrain.west_of(tc)))
					.classify();
				if class < north_west && (north_west != northern && north_west != western) {
					self.images
						.terrain_batches
						.tile_sprite(north_west)
						.add(param);
					let param_rot = param
						.rotation(-std::f32::consts::PI / 2.)
						.dest(dest + logic::glm::vec2(0., screen_size));
					self.images
						.terrain_batches
						.tile_mask_c1(north_west)
						.add(param_rot);
				}
			}
		}

		// The Mask itself is draw multiplicative
		self.terrain_transition_mask_canvas
			.set_blend_mode(Some(BlendMode::Multiply));

		let mask_canvas = &self.terrain_transition_mask_canvas;
		let trans_canvas = &self.terrain_transition_canvas;

		fn draw_mask_n_tiles(
			ctx: &mut gwg::Context,
			quad_ctx: &mut gwg::miniquad::Context,
			mask_canvas: &Canvas,
			trans_canvas: &Canvas,
			mask: Vec<&mut SpriteBatch>,
			tile: &mut SpriteBatch,
		) -> GameResult {
			// The mask canvas, needs to be cleared with white
			graphics::set_canvas(ctx, Some(mask_canvas));
			graphics::clear(ctx, quad_ctx, [1.0, 1.0, 1.0, 0.0].into());

			// Drawing the mask
			draw_and_clear(ctx, quad_ctx, mask)?;

			// The Tile canvas
			graphics::set_canvas(ctx, Some(trans_canvas));
			graphics::clear(ctx, quad_ctx, [0.0, 0.0, 0.0, 0.0].into());

			// Drawing the tile
			draw_and_clear(ctx, quad_ctx, [tile])?;

			// And multiplying the mask on top
			graphics::draw(ctx, quad_ctx, mask_canvas, (Point2::new(0., 0.),))?;

			// Switch back to the screen
			graphics::set_canvas(ctx, None);

			// Draw the transition tiles
			graphics::draw(ctx, quad_ctx, trans_canvas, (Point2::new(0., 0.),))
		}

		// Draw and clear sprite batches
		// This here defines the draw order.

		let res = &mut self.images.resource_batches;
		let tiles = &mut self.images.terrain_batches;

		// Start with the deep tiles
		draw_and_clear(ctx, quad_ctx, [&mut tiles.deep])?;

		// Then the shallow water tiles
		let (tile, mask) = tiles.shallow_batches();
		draw_mask_n_tiles(ctx, quad_ctx, mask_canvas, trans_canvas, mask, tile)?;

		// Then fishies, and other doodads, as well as the wave layer
		draw_and_clear(
			ctx,
			quad_ctx,
			[].into_iter()
				.chain(res.starfishes.iter_mut().map(DerefMut::deref_mut))
				.chain(res.fishes.iter_mut().map(DerefMut::deref_mut))
				.chain(res.shoe.iter_mut().map(DerefMut::deref_mut))
				.chain([&mut tiles.water_anim, &mut tiles.water_anim_2]),
		)?;

		// Then the beaches
		let (tile2, mask2) = tiles.beach_batches();
		draw_mask_n_tiles(ctx, quad_ctx, mask_canvas, trans_canvas, mask2, tile2)?;

		// Just above them the sea grass
		draw_and_clear(ctx, quad_ctx, res.grass.iter_mut().map(DerefMut::deref_mut))?;

		// And finally the grass land tiles
		let (tile3, mask3) = tiles.grass_batches();
		draw_mask_n_tiles(ctx, quad_ctx, mask_canvas, trans_canvas, mask3, tile3)?;

		// Then above all, the harbor and the player's ship
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

		// Draw UI elements
		self.draw_ui(glob, ctx, quad_ctx)?;

		// Draw FPS, right top corner
		let fps = timer::fps(ctx);
		let fps_display = Text::new(format!("FPS: {:.0}", fps));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&fps_display,
			(
				Point2::new(screen_coords.w - fps_display.width(ctx), 0.0),
				Color::WHITE,
			),
			Color::BLACK,
		)?;

		// Some Developer text
		cfg_if! {
			if #[cfg(feature = "dev")] {
				let left_margin = 150.;

				// Current input state
				let input_text = Text::new(format!("Input: {:?}", self.input));
				self.draw_text_with_halo(
					ctx,
					quad_ctx,
					&input_text,
					(Point2::new(left_margin, 20.0), Color::WHITE),
					Color::BLACK,
				)?;

				// Current Wind
				let input_text = Text::new(format!(
					"Wind: {:.2} m/s, {:.0}°",
					self.world.state.wind.magnitude(),
					self.world.state.wind.angle().to_degrees(),
				));
				self.draw_text_with_halo(
					ctx,
					quad_ctx,
					&input_text,
					(Point2::new(left_margin, 40.0), Color::WHITE),
					Color::BLACK,
				)?;
				self.draw_text_with_halo(
					ctx,
					quad_ctx,
					&Text::new("→"),
					(
						Point2::new(90.0, 60.0),
						self.world.state.wind.angle(),
						Color::WHITE,
					),
					Color::BLACK,
				)?;

				// Current Ship states
				let input_text = Text::new(format!(
					"Ship: {:.1} m/s, fish: {:} kg / {:} ℓ",
					self.world.state.player.vehicle.ground_speed(),
					self.world.state.player.vehicle.resource_weight,
					self.world.state.player.vehicle.resource_value,
				));
				self.draw_text_with_halo(
					ctx,
					quad_ctx,
					&input_text,
					(Point2::new(left_margin, 60.0), Color::WHITE),
					Color::BLACK,
				)?;

				// Current Ship states
				let input_text = Text::new(format!(
					"Ori: {:.0}°, List: {:.0}°",
					self.world
						.state
						.player
						.vehicle
						.heading
						.rem_euclid(std::f32::consts::TAU).to_degrees(),
					self.world.state.player.vehicle.angle_of_list.to_degrees()
				));
				self.draw_text_with_halo(
					ctx,
					quad_ctx,
					&input_text,
					(Point2::new(left_margin, 80.0), Color::WHITE),
					Color::BLACK,
				)?;
			}
		}

		let player_loc = self.world.state.player.vehicle.pos;
		let ppm = self.pixel_per_meter(ctx);

		let budget = self.world.state.player.money;
		let value = self.world.state.player.vehicle.resource_value;

		// The trading "interface"
		if let Some(mut t) = self.world.state.get_trading(&self.world.init) {
			let text_color = Color::new(1.0, 1.0, 1.0, 0.85);
			let inactive_color = Color::new(1.0, 1.0, 1.0, 0.4);

			let harbor_dist = self
				.world
				.init
				.terrain
				.torus_distance(player_loc, t.get_harbor().loc);
			let player_loc_sc = nalgebra::Point2::new(screen_coords.w, screen_coords.h) * 0.5;
			let harbor_loc_sc = nalgebra::Point2::from(harbor_dist.0 * ppm + player_loc_sc.coords);
			if t.has_player_valid_speed() {
				// Trading is possible

				let message = {
					let hull_upgrade = t.get_price_of_hull_upgrade();
					let sail_upgrade = t.get_price_for_sail_upgrade();

					match (hull_upgrade, sail_upgrade, t.players_fish_amount()) {
						(Some(hup), _, _) if budget >= hup => "Time to upgrade!".to_owned(),
						(_, Some(sup), _) if budget >= sup => "Time to upgrade!".to_owned(),
						(_, _, fam) if fam > 0 => "Fishy trade?".to_owned(),
						_ => "Time to fish or cut bait!".to_owned(),
					}
				};

				let mut text = Text::new(format!("\"{message}\""));
				text.set_font(Default::default(), PxScale::from(32.));
				let mut offset = 0.0;
				graphics::draw(
					ctx,
					quad_ctx,
					&text,
					(
						Point2::new(
							harbor_loc_sc.x - text.width(ctx) * 0.5,
							harbor_loc_sc.y - text.height(ctx),
						),
						text_color,
					),
				)?;
				offset += text.height(ctx);

				let sell_color = if t.players_fish_amount() > 0 {
					text_color
				} else {
					inactive_color
				};
				let mut sell_text = Text::new(format!("E: Sell fish for {value} €"));
				sell_text.set_font(Default::default(), PxScale::from(20.));

				let (sail_color, sail_message) = if let Some(price) = t.get_price_for_sail_upgrade()
				{
					let c = if budget >= price {
						text_color
					} else {
						inactive_color
					};

					(c, format!("R: Upgrade sail ({price} €)"))
				} else {
					(inactive_color, "Your sail is awesome!".to_owned())
				};
				let mut sail_text = Text::new(sail_message);
				sail_text.set_font(Default::default(), PxScale::from(20.));

				let (hull_color, hull_message) = if let Some(price) = t.get_price_of_hull_upgrade()
				{
					let c = if budget >= price {
						text_color
					} else {
						inactive_color
					};

					(c, format!("F: Upgrade hull ({price} €)"))
				} else {
					(inactive_color, "Your hull is awesome!".to_owned())
				};
				let mut hull_text = Text::new(hull_message);
				hull_text.set_font(Default::default(), PxScale::from(20.));

				let x_offset = sell_text
					.width(ctx)
					.max(sail_text.width(ctx))
					.max(hull_text.width(ctx))
					* 0.5;
				graphics::draw(
					ctx,
					quad_ctx,
					&sell_text,
					(
						Point2::new(
							harbor_loc_sc.x - x_offset,
							harbor_loc_sc.y - sell_text.height(ctx) + offset,
						),
						sell_color,
					),
				)?;
				offset += sell_text.height(ctx) * 1.3;

				graphics::draw(
					ctx,
					quad_ctx,
					&sail_text,
					(
						Point2::new(
							harbor_loc_sc.x - x_offset,
							harbor_loc_sc.y - sail_text.height(ctx) + offset,
						),
						sail_color,
					),
				)?;
				offset += sail_text.height(ctx) * 1.3;

				graphics::draw(
					ctx,
					quad_ctx,
					&hull_text,
					(
						Point2::new(
							harbor_loc_sc.x - x_offset,
							harbor_loc_sc.y - hull_text.height(ctx) + offset,
						),
						hull_color,
					),
				)?;
			} else {
				// Player is too fast for trading

				let mut text = Text::new(
					if t.players_fish_amount() > 0 {
						"\"Slow down, sailor!\""
					} else {
						"\"Time to fish or cut bait!\""
					},
				);
				text.set_font(Default::default(), PxScale::from(32.));
				graphics::draw(
					ctx,
					quad_ctx,
					&text,
					(
						Point2::new(
							harbor_loc_sc.x - text.width(ctx) * 0.5,
							harbor_loc_sc.y - text.height(ctx),
						),
						text_color,
					),
				)?;
			}
		}

		// Finally, issue the draw call and what not, finishing this frame for good
		gwg::graphics::present(ctx, quad_ctx)?;

		Ok(())
	}

	fn key_down_event(
		&mut self,
		glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		keycode: gwg::miniquad::KeyCode,
	) {
		let audios = glob.audios.as_mut().unwrap();

		// Zoom management
		if keycode == KeyCode::KpAdd || keycode == KeyCode::PageUp {
			self.zoom_factor_exp = self.zoom_factor_exp.saturating_add(1);
		}
		if keycode == KeyCode::KpSubtract || keycode == KeyCode::PageDown {
			self.zoom_factor_exp = self.zoom_factor_exp.saturating_sub(1);
		}
		if keycode == KeyCode::Kp0 || keycode == KeyCode::Key0 || keycode == KeyCode::Backspace {
			self.zoom_factor_exp = DEFAULT_ZOOM_LEVEL;
		}

		// Trading interactions.
		// Check whether the player is at a harbor
		if let Some(mut t) = self.world.state.get_trading(&self.world.init) {
			if t.has_player_valid_speed() {
				// Check for sail upgrade key
				if keycode == KeyCode::R {
					let n = t.upgrade_sail();
					match n {
						Ok(()) => {
							// success
							if audios.sound_enabled {
								audios.upgrade_sound.play(ctx).unwrap();
							}
						},
						Err(e) => {
							// Failed
							println!("Failed to upgrade sail: {e}");
							if audios.sound_enabled {
								audios.fail_sound.play(ctx).unwrap();
							}
						},
					}
				}

				// Check for hull upgrade key
				if keycode == KeyCode::F {
					let n = t.upgrade_hull();
					match n {
						Ok(()) => {
							// success
							if audios.sound_enabled {
								audios.upgrade_sound.play(ctx).unwrap();
							}
						},
						Err(e) => {
							// Failed
							println!("Failed to upgrade sail: {e}");
							if audios.sound_enabled {
								audios.fail_sound.play(ctx).unwrap();
							}
						},
					}
				}
			}
		}

		// Reefing input
		if keycode == KeyCode::Up || keycode == KeyCode::W {
			self.input.reefing = self.input.reefing.increase();

			// Limit reefing
			let max_reefing = self.world.state.player.vehicle.sail.kind.max_reefing();
			if self.input.reefing > max_reefing {
				self.input.reefing = max_reefing;
			}
		}
		if keycode == KeyCode::Down || keycode == KeyCode::S {
			self.input.reefing = self.input.reefing.decrease();
		}

		// Sound & Music management
		if keycode == KeyCode::Key1 {
			audios.enable_sound(ctx, !audios.sound_enabled).unwrap();
		}
		if keycode == KeyCode::Key2 {
			audios.enable_music(ctx, !audios.music_enabled).unwrap();
		}

		// Full screen key
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
		let player_loc = self
			.world
			.init
			.terrain
			.map_loc_on_torus(self.world.state.player.vehicle.pos);


		// -- Wind indicator --

		let normed_wind_speed = self.world.state.wind.magnitude() / logic::MAX_WIND_SPEED;
		let n_colors = self.images.ui.wind_speed_colors.len();
		let color_idx_f32 = n_colors.saturating_sub(1) as f32 * normed_wind_speed;
		let color_idx1 = color_idx_f32 as usize;
		let color_idx2 = (color_idx1 + 1).min(n_colors.saturating_sub(1));
		let mix_factor = color_idx_f32.fract();

		let color1 = &self.images.ui.wind_speed_colors[color_idx1];
		let color2 = &self.images.ui.wind_speed_colors[color_idx2];

		let color = color1.mix(color2, mix_factor);
		let padding = 128.;

		// Draw additional info text
		let text_height = {
			cfg_if! {
				if #[cfg(feature = "dev")] {
					let mut wind_text = Text::new(format!(
						"{:.1} m/s, {:.0}°",
						self.world.state.wind.magnitude(),
						self.world.state.wind.angle()
							.rem_euclid(std::f32::consts::TAU)
							.to_degrees(),
					));
					wind_text.set_font(Default::default(), PxScale::from(20.));

					let p = DrawParam::new()
						.dest(Point2::new(
							screen_coords.w - padding - wind_text.width(ctx) * 0.5,
							screen_coords.h - wind_text.height(ctx) - 5.,
						))
						.color(color);
					self.draw_text_with_halo(ctx, quad_ctx, &wind_text, p, Color::BLACK)?;

					wind_text.height(ctx)
				} else {
					0.
				}
			}
		};

		// Draw wind indicator arrow
		let p = DrawParam::new()
			.dest(Point2::new(
				screen_coords.w - padding,
				screen_coords.h - padding - text_height,
			))
			.offset(Point2::new(0.5, 0.5))
			.color(color)
			.scale(logic::glm::vec1(normed_wind_speed).xx())
			.rotation(self.world.state.wind.angle() + std::f32::consts::FRAC_PI_2);
		gwg::graphics::draw(ctx, quad_ctx, &self.images.ui.wind_direction_indicator, p)?;



		// -- Harbor indicators --
		for harbor_distance in self.world.state.harbors.iter().map(|harbor| {
			self.world
				.init
				.terrain
				.torus_distance(player_loc, harbor.loc)
		}) {
			let player_loc_sc = nalgebra::Point2::new(screen_coords.w, screen_coords.h) * 0.5;
			let harbor_loc_sc = nalgebra::Point2::from(
				harbor_distance.0 * self.pixel_per_meter(ctx) + player_loc_sc.coords,
			);

			if !screen_coords.contains(harbor_loc_sc) {
				let towards_harbor = (harbor_loc_sc - player_loc_sc).normalize();
				let harbor_line = Line(player_loc_sc, harbor_loc_sc);

				let screen_corners = [
					nalgebra::Point2::new(screen_coords.x, screen_coords.y + screen_coords.h),
					nalgebra::Point2::new(
						screen_coords.x + screen_coords.w,
						screen_coords.y + screen_coords.h,
					),
					nalgebra::Point2::new(screen_coords.x + screen_coords.w, screen_coords.y),
					nalgebra::Point2::new(screen_coords.x, screen_coords.y),
				];

				let display_point = (0..screen_corners.len())
					.map(|idx1: usize| {
						let idx2 = (idx1 + 1) % screen_corners.len();
						Line(screen_corners[idx1], screen_corners[idx2])
					})
					.filter_map(|line| harbor_line.intersect(&line))
					.filter(|intersection_point| {
						screen_coords.contains(intersection_point - towards_harbor * 0.01)
					})
					.min_by(|a, b| {
						let dst_a = logic::glm::distance2(&harbor_loc_sc.coords, &a.coords);
						let dst_b = logic::glm::distance2(&harbor_loc_sc.coords, &b.coords);
						dst_a.partial_cmp(&dst_b).unwrap()
					});

				if let Some(point) = display_point {
					let inset = self.images.ui.harbor_indicator.width() as f32;
					let draw_point = nalgebra::Point2::new(
						point.x.clamp(
							screen_coords.x + inset,
							screen_coords.x + screen_coords.w - inset,
						),
						point.y.clamp(
							screen_coords.y + inset,
							screen_coords.y + screen_coords.h - inset,
						),
					);
					let max_dist = self.map_length() * 0.5;
					let harbor_dst = harbor_distance.magnitude();
					let harbor_closeness = (max_dist - harbor_dst).max(0.0) / max_dist;

					let mut p = DrawParam::new()
						.dest(draw_point)
						.offset(Point2::new(0.5, 0.5));
					p.color.a = harbor_closeness;
					gwg::graphics::draw(ctx, quad_ctx, &self.images.ui.harbor_indicator, p)?;

					let mut text = Text::new(format!("{}m", harbor_distance.magnitude().round()));
					text.set_font(Default::default(), PxScale::from(18.));
					graphics::draw(
						ctx,
						quad_ctx,
						&text,
						(
							Point2::new(draw_point.x - text.width(ctx) * 0.5, draw_point.y),
							p.color,
						),
					)?;
				}
			}
		}

		// Fishy indicator
		let p = DrawParam::new()
			.dest(Point2::new(0.0, 0.0))
			.offset(Point2::new(-0.25, -0.25))
			.scale(logic::glm::vec2(0.5, 0.5));
		gwg::graphics::draw(ctx, quad_ctx, &self.images.ui.fishy_icon, p)?;

		let mut fishy_text = Text::new(format!(
			"{} kg",
			self.world.state.player.vehicle.resource_weight
		));
		fishy_text.set_font(Default::default(), PxScale::from(32.0));
		let p = DrawParam::new()
			.dest(Point2::new(
				self.images.ui.fishy_icon.width() as f32 * 0.75,
				self.images.ui.fishy_icon.height() as f32 * 0.75 * 0.5
					- fishy_text.height(ctx) as f32 * 0.5,
			))
			.color(Color::WHITE)
			.offset(Point2::new(-0.5, -0.5));
		self.draw_text_with_halo(ctx, quad_ctx, &fishy_text, p, Color::BLACK)?;

		// Money indicator
		let p = DrawParam::new()
			.dest(Point2::new(
				0.0,
				self.images.ui.fishy_icon.height() as f32 * 0.5,
			))
			.offset(Point2::new(-0.25, -0.25))
			.scale(logic::glm::vec2(0.5, 0.5));
		gwg::graphics::draw(ctx, quad_ctx, &self.images.ui.money_icon, p)?;

		let mut money_text = Text::new(format!("{} €", self.world.state.player.money));
		money_text.set_font(Default::default(), PxScale::from(32.0));
		let p = DrawParam::new()
			.dest(Point2::new(
				self.images.ui.money_icon.width() as f32 * 0.75,
				self.images.ui.fishy_icon.height() as f32 * 0.5
					+ self.images.ui.money_icon.height() as f32 * 0.75 * 0.5
					- fishy_text.height(ctx) as f32 * 0.5,
			))
			.color(Color::WHITE)
			.offset(Point2::new(-0.5, -0.5));
		self.draw_text_with_halo(ctx, quad_ctx, &money_text, p, Color::BLACK)?;

		let mut total_height = 0.0;
		for (i, compliment) in self.fished_compliments.iter().enumerate().rev() {
			let mut compliment_text = Text::new(format!("{}. {compliment}", i + 1));
			compliment_text.set_font(Default::default(), PxScale::from(22.0));
			total_height += compliment_text.height(ctx) * 1.3;
			let p = DrawParam::new()
				.dest(Point2::new(40.0, screen_coords.h - total_height - 40.0))
				.color(Color::WHITE)
				.offset(Point2::new(-0.5, -0.5));
			self.draw_text_with_halo(ctx, quad_ctx, &compliment_text, p, Color::BLACK)?;
		}

		let mut compliments_title = Text::new(format!(
			"Fish for compliments! ({}/{})",
			self.fished_compliments.len(),
			COMPLIMENTS.len()
		));
		compliments_title.set_font(Default::default(), PxScale::from(26.0));
		total_height += compliments_title.height(ctx) * 2.0;
		let p = DrawParam::new()
			.dest(Point2::new(40.0, screen_coords.h - total_height - 40.0))
			.color(Color::WHITE)
			.offset(Point2::new(-0.5, -0.5));
		self.draw_text_with_halo(ctx, quad_ctx, &compliments_title, p, Color::BLACK)?;

		Ok(())
	}

	fn map_length(&self) -> f32 {
		(u32::from(self.world.init.terrain.edge_length) * logic::TILE_SIZE) as f32
	}
}
