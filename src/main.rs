use std::env;

use cfg_if::cfg_if;
use good_web_game as gwg;
use gwg::cgmath::Point2;
use gwg::graphics;
use gwg::graphics::Color;
use gwg::graphics::PxScale;
use gwg::graphics::Rect;
use gwg::graphics::Text;
use gwg::GameResult;
use lazy_static::lazy_static;
use logic::DebuggingConf;
use structopt::StructOpt;

mod assets;
mod math;
mod scenes;

#[derive(Debug, Clone)]
#[derive(structopt::StructOpt)]
struct Opts {
	/// Draw bounding boxes
	#[structopt(long)]
	bounding_boxes: bool,

	/// Give the ship an engine, cheat
	#[cfg(feature = "dev")]
	#[structopt(long)]
	engine_cheat: bool,

	/// Let the wind turn predictably, cheat
	#[cfg(feature = "dev")]
	#[structopt(long)]
	wind_turn_cheat: bool,

	/// Let the wind blow from one direction only
	///
	/// You can give the wind direction in radians.
	#[cfg(feature = "dev")]
	#[structopt(long)]
	fixed_wind: Option<f32>,

	/// Specifies the resource factor, cheat.
	#[cfg(feature = "dev")]
	#[structopt(long)]
	resource_factor_cheat: Option<f32>,

	/// Specifies the starting money, cheat.
	#[cfg(feature = "dev")]
	#[structopt(long)]
	money_cheat: Option<u64>,

	/// Disables all sounds and music.
	#[structopt(short, long)]
	muted: bool,

	/// Sets the map size. Bigger maps might reduce performance.
	#[structopt(short = "s", long, default_value = "32")]
	map_size: u16,

	/// Start the game in window modus
	#[structopt(short, long)]
	windowed: bool,

	/// Start the game directly, skipping the main menu
	#[structopt(long)]
	start: bool,

	/// Use a fixed game world seed
	#[structopt(long)]
	seed: Option<String>,
}
impl Opts {
	fn to_debugging_conf(&self) -> logic::DebuggingConf {
		cfg_if! {
			if #[cfg(feature = "dev")] {
				DebuggingConf {
					ship_engine: self.engine_cheat,
					wind_turning: self.wind_turn_cheat,
					fixed_wind_direction: self.fixed_wind,
				}
			} else {
				DebuggingConf {
					.. Default::default()
				}
			}
		}
	}
}

lazy_static! {
	static ref OPTIONS: Opts = Opts::from_args();
}

fn main() -> gwg::GameResult {
	println!("--- [main] entered");

	let opts = &*OPTIONS;

	gwg::start(
		gwg::conf::Conf::default()
			.window_title("Plenty of fish in the sea".into())
			.window_resizable(true)
			.fullscreen(!opts.windowed)
			.cache(Some(include_bytes!(concat!(
				env!("OUT_DIR"),
				"/assets.tar"
			)))),
		|context, quad_ctx| Box::new(scenes::create_stack(context, quad_ctx)),
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
