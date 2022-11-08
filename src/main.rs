use std::env;

use good_web_game as gwg;
use gwg::cgmath::Point2;
use gwg::graphics;
use gwg::graphics::Color;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::Text;
use gwg::GameResult;


mod assets;
mod scenes;



fn main() -> gwg::GameResult {
	gwg::start(
		gwg::conf::Conf::default()
			.window_title("Plenty of fish in the sea".into())
			.window_resizable(true)
			//.fullscreen(true)
			.cache(Some(include_bytes!(concat!(
				env!("OUT_DIR"),
				"/assets.tar"
			)))),
		|context, quad_ctx| Box::new(scenes::Game::new(context, quad_ctx).unwrap()),
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
