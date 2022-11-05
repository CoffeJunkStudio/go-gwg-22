use std::env;

use good_web_game as gwg;
use gwg::audio;
use gwg::cgmath::Point2;
use gwg::graphics::Color;
use gwg::graphics::DrawParam;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::Text;
use gwg::graphics::Transform;
use gwg::graphics::{self,};
use gwg::miniquad::KeyCode;
use gwg::timer;
use gwg::GameResult;
use logic::generator::Generator;
use logic::generator::PerlinNoise;
use logic::generator::Setting;
use logic::state::TICKS_PER_SECOND;
use logic::terrain::TerrainType;
use logic::terrain::TileCoord;
use logic::units::BiPolarFraction;
use logic::units::Distance;
use logic::units::Location;
use logic::Input;
use logic::World;

// #[derive(Debug)] `audio::Source` dose not implement Debug!
struct Game {
	sprite_batch: graphics::spritebatch::SpriteBatch,
	sound: audio::Source,
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
		let image = graphics::Image::new(ctx, quad_ctx, "img/gwg.png").unwrap();
		let batch = graphics::spritebatch::SpriteBatch::new(image);

		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;

		// Generate world
		let noise = PerlinNoise;
		let settings = Setting {
			edge_length: 32,
			resource_density: 1.0,
		};

		let rng = logic::StdRng::new(1, 42);
		let mut world = noise.generate(&settings, rng);
		world.state.player.vehicle.heading = 1.0;

		let meters_per_pixel = 200.0 / 1920.0;

		let s = Game {
			sprite_batch: batch,
			sound,
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
}

impl gwg::event::EventHandler for Game {
	fn update(
		&mut self,
		ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		while gwg::timer::check_update_time(ctx, TICKS_PER_SECOND.into()) {
			let _ = self.world.state.update(&self.world.init, &self.input);
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
					let color = match tile {
						TerrainType::Deep => Color::new(0.0, 0.0, 0.2, 1.0),
						TerrainType::Shallow => Color::new(0.0, 0.3, 0.8, 1.0),
						TerrainType::Land => Color::new(0.0, 0.8, 0.2, 1.0),
					};

					let loc = tc.to_location() - player_pos;
					let sprite_pos = loc.0 / self.meters_per_pixel
						+ logic::glm::vec2(screen_coords.w, screen_coords.h) * 0.5;

					let scale = logic::TILE_SIZE as f32 / self.meters_per_pixel / 256.0;
					let param = DrawParam::new()
						.dest(nalgebra::Point2::new(sprite_pos.x, sprite_pos.y))
						.color(color)
						.scale(logic::glm::vec2(scale, scale));

					self.sprite_batch.add(param);
				}
			}
		}

		gwg::graphics::draw(ctx, quad_ctx, &self.sprite_batch, (Point2::new(0.0, 0.0),))?;
		self.sprite_batch.clear();


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


		let mut text = Text::new("Press 'A' for a sound, and Enter to clear");
		text.set_font(Default::default(), PxScale::from(32.));
		graphics::draw(ctx, quad_ctx, &text, (Point2::new(0.0, 30.0), Color::BLACK))?;
		let mut text = Text::new(format!("»{}«", self.input_text.as_str()));
		text.set_font(Default::default(), PxScale::from(64.));
		graphics::draw(ctx, quad_ctx, &text, (Point2::new(0.0, 50.0), Color::BLACK))?;

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
			self.input.reefing = self.input.reefing.increase();
		} else if keycode == KeyCode::Down {
			self.input.reefing = self.input.reefing.decrease();
		}

		// Rudder input
		if keycode == KeyCode::Left {
			self.input.rudder = BiPolarFraction::from_f32(-1.).unwrap();
		} else if keycode == KeyCode::Right {
			self.input.rudder = BiPolarFraction::from_f32(1.).unwrap();
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
