mod in_game;
mod loading;
mod main_menu;


use good_web_game::event;
use good_web_game::event::EventHandler;
use good_web_game::goodies::scene::SceneStack;
use good_web_game::Context;
use good_web_game::GameError;
pub use in_game::Game;

use self::loading::LoadableFn;
use self::loading::Loading;
use crate::assets::audio::Audios;
use crate::scenes::main_menu::MainMenu;


/// Some global state (between the scenes)
struct GlobalState {
	audios: Option<Audios>,
}

fn start_game(
	glob: &mut GlobalState,
	ctx: &mut Context,
	quad_ctx: &mut event::GraphicsContext,
) -> Game {
	Game::new(glob, ctx, quad_ctx).unwrap()
}
fn start_main_menu(
	glob: &mut GlobalState,
	ctx: &mut Context,
	quad_ctx: &mut event::GraphicsContext,
) -> MainMenu {
	/* Old hack, used to be necessary, but handling the resize events, made
	 * this hack obsolete
	 *
	// Set Full screen mode again, if requested, to correctly apply it.
	// Appears buggy if not done here again.
	if !crate::OPTIONS.windowed {
		graphics::set_fullscreen(quad_ctx, true);
	}
	*/

	MainMenu::new(glob, ctx, quad_ctx).unwrap()
}

/// Creates a fresh scene stack with the default scene
///
/// Using an `impl` return type is useful for type inference and hides our global state.
pub fn create_stack(
	ctx: &mut Context,
	_quad_ctx: &mut miniquad::Context,
) -> impl EventHandler<GameError> {
	let mut stack = SceneStack::new(
		ctx,
		GlobalState {
			audios: None,
		},
	);

	stack.push(Box::new(Loading::from(LoadableFn::new(start_main_menu))));

	stack
}
