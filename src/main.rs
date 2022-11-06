use std::env;
use std::ops::DerefMut;
use std::path::Path;
use std::path::PathBuf;

use asset_batch::image_batch;
use asset_batch::AssetBatch;
use asset_config::AssetConfig;
use asset_config::SingleAssetConfig;
use good_web_game as gwg;
use gwg::audio;
use gwg::cgmath::Point2;
use gwg::graphics;
use gwg::graphics::spritebatch::SpriteBatch;
use gwg::graphics::spritebatch::SpriteIdx;
use gwg::graphics::Color;
use gwg::graphics::DrawParam;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::Text;
use gwg::graphics::Transform;
use gwg::miniquad::KeyCode;
use gwg::timer;
use gwg::GameResult;
use logic::generator::Generator;
use logic::generator::PerlinNoise;
use logic::generator::Setting;
use logic::state::Event;
use logic::state::TICKS_PER_SECOND;
use logic::terrain::TileCoord;
use logic::units::BiPolarFraction;
use logic::units::Distance;
use logic::units::Location;
use logic::Input;
use logic::World;
use rand::seq::SliceRandom;
use rand::Rng;

pub mod asset_batch;

const ASSET_CONFIG_STR: &str =
	include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/render_assets.toml"));

struct TerrainBatches {
	deep: SpriteBatch,
	shallow: SpriteBatch,
	land: SpriteBatch,
}

struct ShipSprites {
	body: AssetBatch,
	sail: Vec<AssetBatch>,
}

struct ShipBatches {
	basic: ShipSprites,
}

struct ResourceBatches {
	fishes: Vec<AssetBatch>,
}

struct BuildingBatches {
	harbor: AssetBatch,
}

fn draw_and_clear<'a>(
	ctx: &mut gwg::Context,
	quad_ctx: &mut gwg::miniquad::Context,
	batches: impl IntoIterator<Item = &'a mut SpriteBatch>,
) -> GameResult<()> {
	for batch in batches {
		// For some ridiculous, empty sprite batches cause sever glitches (UB-like) on windows.
		// Thus we will only draw those that aren't empty.
		if !batch.get_sprites().is_empty() {
			gwg::graphics::draw(ctx, quad_ctx, batch, (Point2::new(0.0, 0.0),))?;
			batch.clear();
		}
	}

	Ok(())
}

// #[derive(Debug)] `audio::Source` dose not implement Debug!
struct Game {
	sprite_batch: SpriteBatch,
	terrain_batches: TerrainBatches,
	ship_batches: ShipBatches,
	resource_batches: ResourceBatches,
	building_batches: BuildingBatches,
	sound: audio::Source,
	sound_fishy: audio::Source,
	input_text: String,
	full_screen: bool,
	world: World,
	input: Input,
	meters_per_pixel: f32,
}

impl Game {
	fn new(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::GraphicsContext,
	) -> gwg::GameResult<Self> {
		// TODO: make it configurable or randomize (e.g. use an timestamp),
		//       or implement both.
		let seed: u64 = 42;

		let render_config: AssetConfig = toml::from_str(ASSET_CONFIG_STR).unwrap();

		let batch = image_batch(ctx, quad_ctx, "img/gwg.png")?;

		let terrain_batches = TerrainBatches {
			deep: image_batch(ctx, quad_ctx, "img/deepwater0.png")?,
			shallow: image_batch(ctx, quad_ctx, "img/shallowwater.png")?,
			land: image_batch(ctx, quad_ctx, "img/gwg.png")?,
		};

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

		let resource_batches = ResourceBatches {
			fishes: [
				"fish-00", "fish-01", "fish-02", "fish-03", "fish-04", "fish-05", "fish-06",
				"fish-07",
			]
			.into_iter()
			.map(|name| AssetBatch::from_config(ctx, quad_ctx, &render_config, name).unwrap())
			.collect(),
		};

		let building_batches = BuildingBatches {
			harbor: AssetBatch::from_config(ctx, quad_ctx, &render_config, "harbour-00").unwrap(),
		};

		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;
		let sound_fishy = audio::Source::new(ctx, "/sound/fischie.ogg")?;

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


		let meters_per_pixel = 30.0 / 1920.0;

		let s = Game {
			sprite_batch: batch,
			terrain_batches,
			ship_batches,
			resource_batches,
			building_batches,
			sound,
			sound_fishy,
			input_text: String::new(),
			full_screen: false,
			world,
			input: Input::default(),
			meters_per_pixel,
		};

		Ok(s)
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
		let sprite_pos = loc.0 / self.meters_per_pixel
			+ logic::glm::vec2(screen_coords.w, screen_coords.h) * 0.5;

		nalgebra::Point2::new(sprite_pos.x, sprite_pos.y)
	}
}

impl gwg::event::EventHandler for Game {
	fn update(
		&mut self,
		ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
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
			self.input.rudder = BiPolarFraction::default();

			for ev in events {
				match ev {
					Event::Fishy => self.sound_fishy.play(ctx).unwrap(),
				}
			}
		}

		Ok(())
	}

	fn draw(
		&mut self,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		let elapsed = gwg::timer::time_since_start(ctx).as_secs_f32();

		let player_pos = self.world.state.player.vehicle.pos;
		let screen_coords = gwg::graphics::screen_coordinates(ctx);

		let (left_top, right_bottom) = {
			let scm_x = screen_coords.w * self.meters_per_pixel;
			let scm_y = screen_coords.h * self.meters_per_pixel;
			let dst = Distance::new(scm_x * 0.5, scm_y * 0.5);

			let lt = TileCoord::from((player_pos - dst).max(Location::ORIGIN));
			let rb = TileCoord::from(player_pos + dst);

			(lt, rb)
		};

		let red = elapsed.sin() * 0.5 + 0.5;
		let green = (1.3 + elapsed + 0.3).sin() * 0.5 + 0.5;
		let blue = (1.13 * elapsed + 0.7).sin() * 0.5 + 0.5;
		gwg::graphics::clear(ctx, quad_ctx, [red, green, blue, 1.0].into());

		for x in left_top.x.saturating_sub(1)..(right_bottom.x + 1) {
			for y in left_top.y.saturating_sub(1)..(right_bottom.y + 1) {
				let tc = TileCoord::new(x, y);
				if let Some(tile) = self.world.init.terrain.try_get(tc) {
					// if TileCoord::from(self.world.state.player.vehicle.pos) == tc {
					// 	continue;
					// }


					let scale = logic::TILE_SIZE as f32 / self.meters_per_pixel / 256.0;
					let loc = tc.to_location().0; // - logic::glm::vec1(logic::TILE_SIZE as f32).xx() * 0.5;
					let param = DrawParam::new()
						.dest(self.location_to_screen_coords(ctx, Location(loc)))
						.scale(logic::glm::vec2(scale, scale));

					let batch = {
						if tile.0 < -5 {
							&mut self.terrain_batches.deep
						} else if tile.0 < 0 {
							&mut self.terrain_batches.shallow
						} else {
							&mut self.terrain_batches.land
						}
					};

					batch.add(param);
				}
			}
		}

		let ship_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE
				/ self.meters_per_pixel
				/ self.ship_batches.basic.body.params().width as f32,
		)
		.xx();
		let ship_pos = self.world.state.player.vehicle.pos.0
			- logic::glm::vec1(2.5 * logic::VEHICLE_SIZE as f32).xx() * 0.5;
		let param = DrawParam::new()
			.dest(self.location_to_screen_coords(ctx, Location(ship_pos)))
			.scale(ship_scale);
		let heading = f64::from(self.world.state.player.vehicle.heading);
		self.ship_batches.basic.body.add_frame(
			0.0,
			-heading + std::f64::consts::PI,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			param,
		);

		let max_sail = self.ship_batches.basic.sail.len() - 1;
		let sail_reefing = match self.world.state.player.vehicle.sail.reefing {
			logic::state::Reefing::Reefed(n) => n,
		};

		let sail_ass = &mut self.ship_batches.basic.sail[usize::from(sail_reefing).min(max_sail)];
		let sail_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE / self.meters_per_pixel / sail_ass.params().width as f32,
		)
		.xx();
		let sail_param = DrawParam::new()
			.dest(self.location_to_screen_coords(ctx, Location(ship_pos)))
			.scale(sail_scale);
		let sail_orient = f64::from(self.world.state.player.vehicle.sail.orientation);

		let sail_ass = &mut self.ship_batches.basic.sail[usize::from(sail_reefing).min(max_sail)];
		sail_ass.add_frame(
			heading + sail_orient,
			-heading + std::f64::consts::PI,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			sail_param,
		);

		for resource in &self.world.state.resources {
			let mut rng = logic::StdRng::new(
				(resource.loc.0.x * 100.0) as u128,
				(resource.loc.0.y * 100.0) as u128,
			);

			let resource_pos =
				resource.loc.0 - logic::glm::vec1(logic::RESOURCE_PACK_FISH_SIZE as f32).xx() * 0.5;
			let dest = self.location_to_screen_coords(ctx, Location(resource_pos));

			let batch = self.resource_batches.fishes.choose_mut(&mut rng).unwrap();

			let resource_scale = logic::glm::vec1(
				logic::RESOURCE_PACK_FISH_SIZE
					/ self.meters_per_pixel
					/ batch.params().width as f32,
			)
			.xx();
			let param = DrawParam::new().dest(dest).scale(resource_scale);

			batch.add_frame(0.0, rng.gen::<f64>() * std::f64::consts::TAU, 0.0, param);
		}

		for harbor in &self.world.state.harbors {
			let harbor_scale = logic::glm::vec1(
				logic::HARBOR_SIZE
					/ self.meters_per_pixel
					/ self.building_batches.harbor.params().width as f32,
			)
			.xx();
			let harbor_pos =
				harbor.loc.0 - logic::glm::vec1(logic::RESOURCE_PACK_FISH_SIZE as f32).xx() * 0.5;
			let param = DrawParam::new()
				.dest(self.location_to_screen_coords(ctx, Location(harbor_pos)))
				.scale(harbor_scale);

			self.building_batches
				.harbor
				.add_frame(0.0, f64::from(harbor.orientation), 0.0, param);
		}

		draw_and_clear(
			ctx,
			quad_ctx,
			[
				&mut self.terrain_batches.deep,
				&mut self.terrain_batches.shallow,
				&mut self.terrain_batches.land,
			]
			.into_iter()
			.chain(
				self.resource_batches
					.fishes
					.iter_mut()
					.map(DerefMut::deref_mut),
			)
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
			"Ori: {:.2}, {:.2}",
			self.world.state.player.vehicle.heading,
			f32::atan2(
				self.world.state.player.vehicle.heading_vec().y,
				self.world.state.player.vehicle.heading_vec().x
			)
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 80.0), Color::WHITE),
			Color::BLACK,
		)?;


		let mut text = Text::new("Press 'A' for a sound, and Enter to clear");
		text.set_font(Default::default(), PxScale::from(32.));
		graphics::draw(ctx, quad_ctx, &text, (Point2::new(0.0, 300.), Color::BLACK))?;
		let mut text = Text::new(format!("»{}«", self.input_text.as_str()));
		text.set_font(Default::default(), PxScale::from(64.));
		graphics::draw(ctx, quad_ctx, &text, (Point2::new(0.0, 350.), Color::BLACK))?;

		// Print version info
		draw_version(ctx, quad_ctx)?;

		gwg::graphics::present(ctx, quad_ctx)?;

		Ok(())
	}

	fn key_down_event(
		&mut self,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		keycode: gwg::miniquad::KeyCode,
		_keymods: gwg::event::KeyMods,
		_repeat: bool,
	) {
		if keycode == KeyCode::Escape {
			gwg::event::quit(ctx);
		}

		if keycode == KeyCode::Enter || keycode == KeyCode::KpEnter {
			self.input_text.clear();

			self.sound.play(ctx).unwrap()
		}

		if keycode == KeyCode::A {
			self.sound.play(ctx).unwrap()
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
			good_web_game::graphics::set_drawable_size(quad_ctx, 600, 480);
		}
	}

	fn text_input_event(
		&mut self,
		_ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
		character: char,
	) {
		self.input_text.push(character);
	}

	fn resize_event(
		&mut self,
		context: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::GraphicsContext,
		w: f32,
		h: f32,
	) {
		//self.screen_width = w;
		//self.screen_height = h;
		let coordinates = graphics::Rect::new(0., 0., w, h);

		graphics::set_screen_coordinates(context, coordinates).expect("Can't resize the window");
	}
}

fn main() -> gwg::GameResult {
	gwg::start(
		gwg::conf::Conf::default()
			.window_title("GWG Prep".into())
			.window_resizable(true)
			//.fullscreen(true)
			.cache(Some(include_bytes!(concat!(
				env!("OUT_DIR"),
				"/assets.tar"
			)))),
		|context, quad_ctx| Box::new(Game::new(context, quad_ctx).unwrap()),
	)
}


/// Draw the built version information
fn draw_version(
	ctx: &mut gwg::Context,
	quad_ctx: &mut gwg::miniquad::Context,
) -> gwg::GameResult<()> {
	// The list of crate features active at compilation
	let features = {
		if !crate::built_info::FEATURES_STR.is_empty() {
			Some(format!("[{}]", crate::built_info::FEATURES_STR))
		} else {
			None
		}
	};
	// The crate version
	let version = {
		let prof = if crate::built_info::PROFILE != "release" {
			format!(" ({})", crate::built_info::PROFILE,)
		} else {
			String::new()
		};
		format!("Version: {}{}", crate::built_info::PKG_VERSION, prof,)
	};
	// The build time (when the binary was compiled)
	let time = {
		{
			crate::built_info::BUILT_TIME_UTC.to_string()
		}
	};
	// More details: commit hash, target, compiler
	let details = {
		let git = {
			if let Some(hash) = crate::built_info::GIT_COMMIT_HASH {
				let dirt = if Some(true) == crate::built_info::GIT_DIRTY {
					" (dirty)"
				} else {
					""
				};
				format!("{hash}{dirt}")
			} else {
				String::new()
			}
		};
		{
			format!(
				"{} - {} - {}",
				git,
				crate::built_info::TARGET,
				crate::built_info::RUSTC_VERSION
			)
		}
	};

	// Get the screen size, for centering text
	let Rect {
		w: screen_width,
		h: screen_height,
		..
	} = graphics::screen_coordinates(ctx);

	// A little helper to draw centered text
	let mut centered_text = |height: f32, t: &str, size: u16| -> GameResult<f32> {
		let mut text = Text::new(t);
		text.set_font(Default::default(), PxScale::from(size as f32));
		text.set_bounds(
			Point2::new(screen_width, f32::INFINITY),
			graphics::Align::Center,
		);
		let text_height = text.dimensions(ctx).h;

		graphics::draw(
			ctx,
			quad_ctx,
			&text,
			(
				Point2::new(0., screen_height - height - text_height),
				Color::BLACK,
			),
		)?;
		graphics::draw(
			ctx,
			quad_ctx,
			&text,
			(
				Point2::new(0. + 1., screen_height - height - text_height + 1.),
				Color::WHITE,
			),
		)?;

		Ok(text_height)
	};

	// Notice, we gona draw from bottom to top!
	let mut h = 0.0;
	h += centered_text(h, &details, 12)?;
	if let Some(f) = features {
		h += centered_text(h, &f, 16)?;
	}
	h += centered_text(h, &time, 16)?;
	centered_text(h, &version, 16)?;

	Ok(())
}

// Build-time information
mod built_info {
	// The file has been placed there by the build script.
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
