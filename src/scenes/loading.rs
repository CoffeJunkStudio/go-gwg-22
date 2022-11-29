use std::marker::PhantomData;

use good_web_game::event::GraphicsContext;
use good_web_game::goodies::scene::Scene;
use good_web_game::goodies::scene::SceneSwitch;
use good_web_game::graphics::Font;
use good_web_game::graphics::Text;
use good_web_game::graphics::{self,};
use good_web_game::Context;
use good_web_game::GameResult;
use nalgebra::Point2;

use super::GlobalState;


const DEFAULT_DELAY: u16 = 3;


/// A scene loader
pub(super) trait Loadable {
	type Target: Scene<GlobalState> + 'static;

	fn load(
		&self,
		glob: &mut GlobalState,
		ctx: &mut Context,
		quad_ctx: &mut GraphicsContext,
	) -> Self::Target;
}

/// An `Fn` wrapper as scene loader
pub struct LoadableFn<T, F> {
	_t: PhantomData<T>,
	f: F,
}
impl<T, F> LoadableFn<T, F> {
	pub fn new(f: F) -> Self {
		Self {
			_t: PhantomData,
			f,
		}
	}
}

impl<
		T: Scene<GlobalState> + 'static,
		F: Fn(&mut GlobalState, &mut Context, &mut GraphicsContext) -> T,
	> From<F> for LoadableFn<T, F>
{
	fn from(f: F) -> Self {
		Self::new(f)
	}
}

impl<
		T: Scene<GlobalState> + 'static,
		F: Fn(&mut GlobalState, &mut Context, &mut GraphicsContext) -> T,
	> Loadable for LoadableFn<T, F>
{
	type Target = T;

	fn load(
		&self,
		glob: &mut GlobalState,
		ctx: &mut Context,
		quad_ctx: &mut GraphicsContext,
	) -> Self::Target {
		(self.f)(glob, ctx, quad_ctx)
	}
}

/// Loads the given scene after a short delay.
pub struct Loading<S> {
	loadable: S,
	delay: u16,
}

impl<S> Loading<S> {
	pub fn new(loadable: S, delay: u16) -> Self {
		Self {
			loadable,
			delay,
		}
	}
}

impl<S: Loadable> From<S> for Loading<S> {
	fn from(loadable: S) -> Self {
		Self::new(loadable, DEFAULT_DELAY)
	}
}

impl<S: Loadable> Scene<GlobalState> for Loading<S> {
	fn update(
		&mut self,
		glob: &mut GlobalState,
		ctx: &mut Context,
		quad_ctx: &mut GraphicsContext,
	) -> SceneSwitch<GlobalState> {
		if self.delay == 0 {
			SceneSwitch::Replace(Box::new(self.loadable.load(glob, ctx, quad_ctx)))
		} else {
			self.delay -= 1;
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

		//graphics::draw(ctx, quad_ctx, &Text::new("Loading ..."), (Point2::new(1.,1.),))?;

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

		let mut loading = Text::new("Loading ...");
		loading.set_font(Font::default(), (2. * Font::DEFAULT_FONT_SCALE).into());
		loading.set_bounds(Point2::new(size.0, size.1), graphics::Align::Center);
		graphics::draw(
			ctx,
			quad_ctx,
			&loading,
			(Point2::new(0., size.1 / 2. + Font::DEFAULT_FONT_SCALE),),
		)
	}

	fn name(&self) -> &str {
		"Loading"
	}
}
