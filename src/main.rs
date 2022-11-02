use std::env;

use good_web_game as gwg;
use gwg::audio;
use gwg::cgmath::Point2;
use gwg::cgmath::Vector2;
use gwg::graphics::Color;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::Text;
use gwg::graphics::{self,};
use gwg::miniquad::KeyCode;
use gwg::timer;
use gwg::GameResult;

// #[derive(Debug)] `audio::Source` dose not implement Debug!
struct Game {
	sprite_batch: graphics::spritebatch::SpriteBatch,
	sound: audio::Source,
	input_text: String,
	full_screen: bool,
}

impl Game {
	fn new(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::GraphicsContext,
	) -> gwg::GameResult<Self> {
		let image = graphics::Image::new(ctx, quad_ctx, "img/gwg.png").unwrap();
		let batch = graphics::spritebatch::SpriteBatch::new(image);

		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;

		let s = Game {
			sprite_batch: batch,
			sound,
			input_text: String::new(),
			full_screen: false,
		};

		Ok(s)
	}

	// From the sprites batch example
	// Source: https://github.com/ggez/good-web-game/blob/master/examples/spritebatch.rs
	fn sprites_batches(
		&mut self,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		let time = (timer::duration_to_f64(timer::time_since_start(ctx)) * 1000.0) as u32;
		let cycle = 10_000;
		for x in 0..100 {
			for y in 0..100 {
				let x = x as f32;
				let y = y as f32;
				let p = graphics::DrawParam::new()
					.dest(Point2::new(x * 10.0, y * 10.0))
					.scale(Vector2::new(
						((time % cycle * 2) as f32 / cycle as f32 * 6.28)
							.cos()
							.abs() * 0.0625,
						((time % cycle * 2) as f32 / cycle as f32 * 6.28)
							.cos()
							.abs() * 0.0625,
					))
					.rotation(-2.0 * ((time % cycle) as f32 / cycle as f32 * 6.28));
				self.sprite_batch.add(p);
			}
		}

		let param = graphics::DrawParam::new()
			.dest(Point2::new(
				((time % cycle) as f32 / cycle as f32 * 6.28).cos() * 50.0 + 150.0,
				((time % cycle) as f32 / cycle as f32 * 6.28).sin() * 50.0 - 150.0,
			))
			.scale(Vector2::new(
				((time % cycle) as f32 / cycle as f32 * 6.28).sin().abs() * 2.0 + 1.0,
				((time % cycle) as f32 / cycle as f32 * 6.28).sin().abs() * 2.0 + 1.0,
			))
			.rotation((time % cycle) as f32 / cycle as f32 * 6.28)
			// applying a src parameter to a sprite batch globally has no effect
			//.src([0.25,0.25,0.5,0.5].into())
			.offset(Point2::new(750.0, 750.0));

		graphics::draw(ctx, quad_ctx, &self.sprite_batch, param)?;
		self.sprite_batch.clear();

		Ok(())
	}
}


impl gwg::event::EventHandler for Game {
	fn update(
		&mut self,
		_ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		Ok(())
	}

	fn draw(
		&mut self,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
	) -> gwg::GameResult<()> {
		let elapsed = gwg::timer::time_since_start(ctx).as_secs_f32();

		let red = elapsed.sin() * 0.5 + 0.5;
		let green = (1.3 + elapsed + 0.3).sin() * 0.5 + 0.5;
		let blue = (1.13 * elapsed + 0.7).sin() * 0.5 + 0.5;
		gwg::graphics::clear(ctx, quad_ctx, [red, green, blue, 1.0].into());


		self.sprites_batches(ctx, quad_ctx)?;


		// From the text example
		// Source: https://github.com/ggez/good-web-game/blob/master/examples/text.rs
		let fps = timer::fps(ctx);
		let fps_display = Text::new(format!("FPS: {:.1}", fps));
		// When drawing through these calls, `DrawParam` will work as they are documented.
		graphics::draw(
			ctx,
			quad_ctx,
			&fps_display,
			(Point2::new(100.0, 0.0), Color::WHITE),
		)?;
		graphics::draw(
			ctx,
			quad_ctx,
			&fps_display,
			(Point2::new(101.0, 1.0), Color::BLACK),
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
