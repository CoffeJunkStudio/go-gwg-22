use std::ops::DerefMut;

use good_web_game as gwg;
use gwg::audio;
use gwg::cgmath::Point2;
use gwg::goodies::scene::Scene;
use gwg::goodies::scene::SceneSwitch;
use gwg::graphics;
use gwg::graphics::draw;
use gwg::graphics::Color;
use gwg::graphics::DrawMode;
use gwg::graphics::DrawParam;
use gwg::graphics::MeshBuilder;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::StrokeOptions;
use gwg::graphics::Text;
use gwg::graphics::Transform;
use gwg::miniquad::KeyCode;
use gwg::timer;
use logic::generator::Generator;
use logic::generator::PerlinNoise;
use logic::generator::Setting;
use logic::glm::vec1;
use logic::glm::Vec2;
use logic::state::Event;
use logic::terrain::TileCoord;
use logic::units::BiPolarFraction;
use logic::units::Distance;
use logic::units::Location;
use logic::Input;
use logic::World;
use logic::TICKS_PER_SECOND;
use logic::TILE_SIZE;

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
const DEFAULT_ZOOM_LEVEL: i32 = 0;



// #[derive(Debug)] `audio::Source` dose not implement Debug!
pub struct Game {
	terrain_batches: TerrainBatches,
	ship_batches: ShipBatches,
	resource_batches: ResourceBatches,
	building_batches: BuildingBatches,
	sound: audio::Source,
	sound_fishy: audio::Source,
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
		// TODO: make it configurable or randomize (e.g. use an timestamp),
		//       or implement both.
		let seed: u64 = 44;

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
			land: image_batch(ctx, quad_ctx, "img/grass.png")?,
			water_anim: image_batch(ctx, quad_ctx, "img/wateranim.png")?,
			water_anim_2: image_batch(ctx, quad_ctx, "img/wateranim2.png")?,
		};

		println!(
			"{:.3} [game] loading ships...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let ship_batches = ShipBatches {
			basic: ShipSprites {
				body: AssetBatch::from_config(ctx, quad_ctx, &render_config, "ship-00")?,
				sail: vec![
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-0")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-1")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-2")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-3")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-4")?,
				],
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
			"{:.3} [game] loading sounds...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;
		let sound_fishy = audio::Source::new(ctx, "/sound/fischie.ogg")?;


		println!(
			"{:.3} [game] generating world...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		// Generate world
		let noise = PerlinNoise;
		let settings = Setting {
			edge_length: 32,
			resource_density: 1.0,
		};

		let mut rng = logic::StdRng::new(0xcafef00dd15ea5e5, seed.into());
		let mut world = noise.generate(&settings, &mut rng);
		world.state.player.vehicle.heading = 1.0;
		world.state.player.vehicle.pos = world.init.terrain.random_passable_location(&mut rng);
		world.init.dbg = crate::OPTIONS.to_debugging_conf();

		let s = Game {
			terrain_batches,
			ship_batches,
			resource_batches,
			building_batches,
			sound,
			sound_fishy,
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

			// TODO: fishies collision boxes
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

			for ev in events {
				match ev {
					Event::Fishy => self.sound_fishy.play(ctx).unwrap(),
				}
			}
		}

		if let Some(mut trade) = self.world.state.get_trading() {
			if is_key_pressed(ctx, KeyCode::S) {
				trade.sell_fish(1);
			}
		}

		if is_key_pressed(ctx, KeyCode::Escape) {
			SceneSwitch::Pop
		} else {
			SceneSwitch::None
		}
	}

	fn draw(
		&mut self,
		_glob: &mut GlobalState,
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
			let scm_x = (screen_coords.w / pixel_per_meter).min(terrain.map_size() - 3.*logic::TILE_SIZE as f32);
			let scm_y = (screen_coords.h / pixel_per_meter).min(terrain.map_size() - 3.*logic::TILE_SIZE as f32);
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
				self.terrain_batches.water_anim.add(param);

				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(wave_1 - quarter_tile)))
					.scale(logic::glm::vec2(scale, scale))
					.color(Color::new(f2, f2, f2, 1.));
				self.terrain_batches.water_anim.add(param);

				// Add the offset
				let wave_2 = loc + self.water_wave_2_offset;

				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(wave_2)))
					.scale(logic::glm::vec2(scale, scale));
				self.terrain_batches.water_anim_2.add(param);
			}
		}

		// Draw the tile background
		for (tc, tile) in terrain.iter() {
			if terrain.torus_bounds_check(left_top, right_bottom, tc.to_location()) {
				let remapped = terrain.torus_remap(left_top, tc.to_location());

				let scale = logic::TILE_SIZE as f32 * pixel_per_meter / tile_image_size;
				let loc = remapped.0 - half_tile;
				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(loc)))
					.scale(logic::glm::vec2(scale, scale));

				let batch = {
					if tile.0 < -5 {
						&mut self.terrain_batches.deep
					} else if tile.0 < 0 {
						&mut self.terrain_batches.shallow
					} else if tile.0 < 1 {
						&mut self.terrain_batches.beach
					} else {
						&mut self.terrain_batches.land
					}
				};

				batch.add(param);
			}
		}

		// Draw the player ship
		let ship_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE * pixel_per_meter
				/ self.ship_batches.basic.body.params().width as f32,
		)
		.xx();
		let ship_pos = self.world.state.player.vehicle.pos.0
			- logic::glm::vec1(2.5 * logic::VEHICLE_SIZE).xx() * 0.5;
		let param = DrawParam::new()
			.dest(self.location_to_screen_coords(ctx, Location(ship_pos)))
			.scale(ship_scale);
		let heading = f64::from(self.world.state.player.vehicle.heading);
		let ship_heading = -heading + std::f64::consts::PI;
		self.ship_batches.basic.body.add_frame(
			0.0,
			ship_heading,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			param,
		);

		// Draw the player sail
		let max_sail = self.ship_batches.basic.sail.len() - 1;
		let sail_reefing = match self.world.state.player.vehicle.sail.reefing {
			logic::state::Reefing::Reefed(n) => n,
		};

		let sail_ass = &mut self.ship_batches.basic.sail[usize::from(sail_reefing).min(max_sail)];
		let sail_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE * pixel_per_meter / sail_ass.params().width as f32,
		)
		.xx();
		let sail_param = DrawParam::new()
			.dest(self.location_to_screen_coords(ctx, Location(ship_pos)))
			.scale(sail_scale);
		let orientation = f64::from(self.world.state.player.vehicle.sail.orientation);
		let sail_orient = -orientation + std::f64::consts::PI;

		let sail_ass = &mut self.ship_batches.basic.sail[usize::from(sail_reefing).min(max_sail)];
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

				let batch = &mut self.resource_batches.fishes[usize::from(resource.variant)];

				let resource_scale = logic::glm::vec1(
					logic::RESOURCE_PACK_FISH_SIZE * pixel_per_meter / batch.params().width as f32,
				)
				.xx();
				let param = DrawParam::new().dest(dest).scale(resource_scale);

				batch.add_frame(0.0, -f64::from(resource.ori), 0.0, param);
			}
		}

		// Draw harbors
		for harbor in &self.world.state.harbors {
			if terrain.torus_bounds_check(left_top, right_bottom, harbor.loc) {
				let remapped = terrain.torus_remap(left_top, harbor.loc);

				let harbor_scale = logic::glm::vec1(
					2. * logic::HARBOR_SIZE * pixel_per_meter
						/ self.building_batches.harbor.params().width as f32,
				)
				.xx();
				let harbor_pos = remapped.0 - logic::glm::vec1(2. * logic::HARBOR_SIZE).xx() * 0.5;
				let param = DrawParam::new()
					.dest(self.location_to_screen_coords(ctx, Location(harbor_pos)))
					.scale(harbor_scale);

				self.building_batches.harbor.add_frame(
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
					&mut self.terrain_batches.deep,
					&mut self.terrain_batches.shallow,
				])
				.chain(
					self.resource_batches
						.fishes
						.iter_mut()
						.map(DerefMut::deref_mut),
				)
				.chain([
					&mut self.terrain_batches.water_anim,
					&mut self.terrain_batches.water_anim_2,
					&mut self.terrain_batches.beach,
					&mut self.terrain_batches.land,
				])
				.chain([
					self.building_batches.harbor.deref_mut(),
					self.ship_batches.basic.body.deref_mut(),
				])
				.chain(
					self.ship_batches
						.basic
						.sail
						.iter_mut()
						.map(DerefMut::deref_mut),
				),
		)?;

		// Draw some debugging stuff
		self.draw_debugging(ctx, quad_ctx)?;

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
			if t.players_fish_amount() > 0 {
				if t.has_player_valid_speed() {
					// Trading is possible

					let mut text = Text::new("Press 'S' to sell fish");
					text.set_font(Default::default(), PxScale::from(32.));
					graphics::draw(
						ctx,
						quad_ctx,
						&text,
						(Point2::new(0.0, screen_coords.h / 2.), Color::BLACK),
					)?;
				} else {
					// Player is too fast for trading

					let mut text = Text::new("Slow down for trading");
					text.set_font(Default::default(), PxScale::from(32.));
					graphics::draw(
						ctx,
						quad_ctx,
						&text,
						(Point2::new(0.0, screen_coords.h / 2.), Color::BLACK),
					)?;
				}
			} else {
				// No fish to sell

				let mut text = Text::new("You need more fish");
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
			self.sound.play(ctx).unwrap()
		}

		if keycode == KeyCode::A {
			self.sound.play(ctx).unwrap()
		}

		if keycode == KeyCode::KpAdd {
			self.zoom_factor_exp = self.zoom_factor_exp.saturating_add(1);
		}
		if keycode == KeyCode::KpSubtract {
			self.zoom_factor_exp = self.zoom_factor_exp.saturating_sub(1);
		}
		if keycode == KeyCode::Kp0 {
			self.zoom_factor_exp = 0;
		}

		// Reefing input
		if keycode == KeyCode::Up {
			// TODO: limit reefing
			self.input.reefing = self.input.reefing.increase();
		} else if keycode == KeyCode::Down {
			self.input.reefing = self.input.reefing.decrease();
		}

		if keycode == KeyCode::F11 {
			self.full_screen = !self.full_screen;
			println!("{}", self.full_screen);
			good_web_game::graphics::set_fullscreen(quad_ctx, self.full_screen);
			//good_web_game::graphics::set_drawable_size(quad_ctx, 600, 480);
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
		context: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::GraphicsContext,
		w: f32,
		h: f32,
	) {
		let coordinates = graphics::Rect::new(0., 0., w, h);

		graphics::set_screen_coordinates(context, coordinates).expect("Can't resize the window");
	}
}
