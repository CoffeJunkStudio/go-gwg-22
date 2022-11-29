use std::marker::PhantomData;

use good_web_game as gwg;
use good_web_game::event;
use good_web_game::event::GraphicsContext;
use good_web_game::goodies::scene::Scene;
use good_web_game::goodies::scene::SceneSwitch;
use good_web_game::graphics::Font;
use good_web_game::graphics::Image;
use good_web_game::graphics::Text;
use good_web_game::graphics::{self,};
use good_web_game::Context;
use good_web_game::GameResult;
use gwg::graphics::DrawParam;
use miniquad::KeyCode;
use nalgebra::Point2;
use nalgebra::Vector2;

use super::Game;
use super::GlobalState;
use super::loading::LoadableFn;
use super::loading::Loading;



/// The main menu or title screen
pub struct MainMenu {
	// todo
	bg: Image,

	first: bool,

	/// Indicates that the game shall begin
	lets_continue : bool,
}

impl MainMenu {
	pub fn new(ctx: &mut Context,
		quad_ctx: &mut miniquad::GraphicsContext,) -> GameResult< Self> {
		let bg = graphics::Image::new(ctx, quad_ctx, "/img/bg-16-9-idx.png")?;

		Ok(Self {
			bg,
			first: true,
			lets_continue: crate::OPTIONS.start,
		})
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


		let scale = (size.0 / 64.).max(size.1 / 36.);
		let params = DrawParam::default().dest(Point2::new(size.0/2.,size.1/2.)).scale(
			Vector2::new(scale,scale)
		).offset(Point2::new(0.5,0.5));

		graphics::draw(ctx, quad_ctx, &self.bg, params)?;

		let mut heading = Text::new("Plenty of Fish in the Sea");
		heading.set_font(Font::default(), (3. * Font::DEFAULT_FONT_SCALE).into());
		heading.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
		let height = heading.dimensions(ctx).h;
		graphics::draw(
			ctx,
			quad_ctx,
			&heading,
			(Point2::new(
				0.,
				size.1 / 2. - Font::DEFAULT_FONT_SCALE - height,
			),),
		)?;

		let mut loading = Text::new("Press key to start ...");
		loading.set_font(Font::default(), (2. * Font::DEFAULT_FONT_SCALE).into());
		loading.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
		graphics::draw(
			ctx,
			quad_ctx,
			&loading,
			(Point2::new(0., size.1 / 2. + Font::DEFAULT_FONT_SCALE),),
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
			good_web_game::event::quit(ctx);
		}

		self.lets_continue = true;
	}

	fn name(&self) -> &str {
		"Main Menu"
	}

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
	}
}
