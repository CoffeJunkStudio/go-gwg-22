use cfg_if::cfg_if;
use good_web_game as gwg;
use good_web_game::event::GraphicsContext;
use good_web_game::goodies::scene::Scene;
use good_web_game::goodies::scene::SceneSwitch;
use good_web_game::graphics;
use good_web_game::graphics::Font;
use good_web_game::graphics::Image;
use good_web_game::graphics::Text;
use good_web_game::Context;
use good_web_game::GameResult;
use gwg::graphics::Color;
use gwg::graphics::DrawParam;
use gwg::timer::time;
use miniquad::KeyCode;
use nalgebra::Point2;
use nalgebra::Vector2;

use super::loading::LoadableFn;
use super::loading::Loading;
use super::GlobalState;
use crate::draw_version;



const TEXT_COLOR: Color = Color::new(0.769, 0.769, 0.769, 1.0); // #c4c4c4
const BUTTON_COLOR: Color = Color::new(0.282, 0.424, 0.557, 1.0); // #486c8e
const VERSION_COLOR: Color = Color::new(0.192, 0.122, 0.373, 1.0); // #311f5f


/// The main menu or title screen
pub struct MainMenu {
	bg: Image,
	key_bg: Image,

	/// Indicates that the game shall begin
	lets_continue: bool,
}

impl MainMenu {
	pub(super) fn new(
		glob: &mut GlobalState,
		ctx: &mut Context,
		quad_ctx: &mut miniquad::GraphicsContext,
	) -> GameResult<Self> {
		let bg = graphics::Image::new(ctx, quad_ctx, "/img/bg-16-9-idx.png")?;
		let key_bg = graphics::Image::new(ctx, quad_ctx, "/img/keybright.png")?;

		if let Some(a) = glob.audios.as_mut() {
			if cfg!(not(target_family = "wasm")) {
				a.enable_music(ctx, !crate::OPTIONS.muted)?;
			}
		}

		Ok(Self {
			bg,
			key_bg,
			lets_continue: crate::OPTIONS.start,
		})
	}

	fn draw_a_button_at_the_center(
		&self,
		ctx: &mut Context,
		quad_ctx: &mut GraphicsContext,
		text: &str,
		offset: (i16, i16),
	) -> GameResult<()> {
		const FONT_SIZE_SCALE: f32 = 2.0;
		const KEY_PADDING_FACTOR: f32 = 1.5;
		const KEY_MARGIN_FACTOR: f32 = 0.1;
		const LABEL_COLOR: Color = BUTTON_COLOR;

		// Screen size
		let size = graphics::drawable_size(quad_ctx);

		let font_size = FONT_SIZE_SCALE * Font::DEFAULT_FONT_SCALE;
		let button_size = KEY_PADDING_FACTOR * font_size;
		let button_distance = (1. + KEY_MARGIN_FACTOR) * button_size;

		// Type set the text
		let mut key_text = Text::new(text);
		key_text.set_font(Font::default(), font_size.into());

		// The global center point of all buttons, jus above the screen center
		let glob_center_point = Point2::new(size.0 / 2., size.1 / 2. - button_size / 2.);
		// The offset for this button
		let offset = Vector2::new(
			button_distance * f32::from(offset.0),
			button_distance * f32::from(offset.1),
		);
		let center_point = glob_center_point + offset;
		// The text offset
		let text_offset = Vector2::new(
			-key_text.width(ctx) / 2. + 1.,
			-key_text.height(ctx) / 2. + 1.,
		);

		// Draw the background image
		let scale = button_size / f32::from(self.key_bg.height());
		let params = DrawParam::default()
			.dest(center_point)
			.scale(Vector2::new(scale, scale))
			.offset(Point2::new(0.5, 0.5));
		graphics::draw(ctx, quad_ctx, &self.key_bg, params)?;

		// Draw the label text
		let params = DrawParam::default()
			.dest(center_point + text_offset)
			.color(LABEL_COLOR);
		graphics::draw(ctx, quad_ctx, &key_text, params)?;

		Ok(())
	}
}

impl Scene<GlobalState> for MainMenu {
	fn update(
		&mut self,
		_glob: &mut GlobalState,
		_ctx: &mut Context,
		_quad_ctx: &mut GraphicsContext,
	) -> SceneSwitch<GlobalState> {
		if self.lets_continue {
			self.lets_continue = false;
			SceneSwitch::Push(Box::new(Loading::from(LoadableFn::new(super::start_game))))
		} else {
			SceneSwitch::None
		}
	}

	fn draw(
		&mut self,
		_glob: &mut GlobalState,
		ctx: &mut Context,
		quad_ctx: &mut GraphicsContext,
	) -> GameResult<()> {
		let size = graphics::drawable_size(quad_ctx);

		graphics::clear(ctx, quad_ctx, [0.0, 0.0, 0.0, 1.0].into());

		// Draw background image
		let scale = (size.0 / 64.).max(size.1 / 36.);
		let params = DrawParam::default()
			.dest(Point2::new(size.0 / 2., size.1 / 2.))
			.scale(Vector2::new(scale, scale))
			.offset(Point2::new(0.5, 0.5));
		graphics::draw(ctx, quad_ctx, &self.bg, params)?;

		// Draw the how-to
		self.draw_a_button_at_the_center(ctx, quad_ctx, "W", (-3, -1))?;
		self.draw_a_button_at_the_center(ctx, quad_ctx, " ", (-4, 0))?;
		self.draw_a_button_at_the_center(ctx, quad_ctx, "S", (-3, 0))?;
		self.draw_a_button_at_the_center(ctx, quad_ctx, " ", (-2, 0))?;

		self.draw_a_button_at_the_center(ctx, quad_ctx, " ", (3, -1))?;
		self.draw_a_button_at_the_center(ctx, quad_ctx, "A", (2, 0))?;
		self.draw_a_button_at_the_center(ctx, quad_ctx, " ", (3, 0))?;
		self.draw_a_button_at_the_center(ctx, quad_ctx, "D", (4, 0))?;

		let mut controls = Text::new("Set Sail            Turning ");
		controls.set_font(Font::default(), (1. * Font::DEFAULT_FONT_SCALE).into());
		controls.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
		let _height = controls.dimensions(ctx).h;
		graphics::draw(
			ctx,
			quad_ctx,
			&controls,
			(
				Point2::new(0., size.1 / 2. + Font::DEFAULT_FONT_SCALE),
				TEXT_COLOR,
			),
		)?;

		// Draw head line
		let mut heading = Text::new("Plenty of Fish in the Sea");
		heading.set_font(Font::default(), (3. * Font::DEFAULT_FONT_SCALE).into());
		heading.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
		let _height = heading.dimensions(ctx).h;
		graphics::draw(
			ctx,
			quad_ctx,
			&heading,
			(Point2::new(0., 3. * Font::DEFAULT_FONT_SCALE), TEXT_COLOR),
		)?;

		// Print version info
		let mut height = draw_version(ctx, quad_ctx, VERSION_COLOR)?;
		let full_option_text_height = (2. + 1. + 2.) * Font::DEFAULT_FONT_SCALE;
		if height + full_option_text_height + 2. * Font::DEFAULT_FONT_SCALE < size.1 / 3. {
			height = size.1 / 3. - full_option_text_height;
		} else {
			height += 2. * Font::DEFAULT_FONT_SCALE;
		}

		// Draw Menu Options
		// Drawing bottom up

		// Show the quit button only on non-WASM platform, because it does not work on WASM
		cfg_if! {
			if #[cfg(not(target_family = "wasm"))] {
				let mut quitting = Text::new("Press Esc to quit");
				quitting.set_font(Font::default(), (2. * Font::DEFAULT_FONT_SCALE).into());
				quitting.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
				height += quitting.height(ctx);
				graphics::draw(
					ctx,
					quad_ctx,
					&quitting,
					(Point2::new(0., size.1 - height + (time().sin() as f32) * 4.), TEXT_COLOR),
				)?;
			}
		}

		// The start button
		let mut starting = Text::new("Press any key to start");
		starting.set_font(Font::default(), (2. * Font::DEFAULT_FONT_SCALE).into());
		starting.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
		height += starting.height(ctx) + Font::DEFAULT_FONT_SCALE;
		graphics::draw(
			ctx,
			quad_ctx,
			&starting,
			(
				Point2::new(0., size.1 - height + (time().cos() as f32) * 4.),
				TEXT_COLOR,
			),
		)?;

		// Finally, issue the draw call and what not, finishing this frame for good
		graphics::present(ctx, quad_ctx)?;

		Ok(())
	}

	fn key_down_event(
		&mut self,
		_gameworld: &mut GlobalState,
		ctx: &mut good_web_game::Context,
		_quad_ctx: &mut miniquad::graphics::GraphicsContext,
		key: good_web_game::event::KeyCode,
	) {
		if key == KeyCode::Escape {
			if cfg!(not(target_family = "wasm")) {
				good_web_game::event::quit(ctx);
			}
		} else {
			self.lets_continue = true;
		}
	}

	fn name(&self) -> &str {
		"Main Menu"
	}

	fn resize_event(
		&mut self,
		_glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::GraphicsContext,
		w: f32,
		h: f32,
	) {
		let coordinates = graphics::Rect::new(0., 0., w, h);

		graphics::set_screen_coordinates(ctx, coordinates).expect("Can't resize the window");
	}
}
