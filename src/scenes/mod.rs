mod in_game;
mod loading;
mod main_menu;


use good_web_game::event::EventHandler;
use good_web_game::event::{self,};
use good_web_game::goodies::scene::{SceneStack, Scene};
use good_web_game::graphics;
use good_web_game::Context;
use good_web_game::GameError;
pub use in_game::Game;

use crate::scenes::main_menu::MainMenu;

use self::loading::LoadableFn;
use self::loading::Loading;


/// Some global state (between the scenes)
struct GlobalState {}

	fn start_game(_: &mut GlobalState, ctx: &mut Context, quad_ctx: &mut event::GraphicsContext) -> Game {

		Game::new(ctx, quad_ctx).unwrap()
	}
	fn start_main_menu(_: &mut GlobalState, ctx: &mut Context, quad_ctx: &mut event::GraphicsContext) -> MainMenu {

		/* Old hack, used to be necessary, but handling the resize events, made
		 * this hack obsolete
		 *
		// Set Full screen mode again, if requested, to correctly apply it.
		// Appears buggy if not done here again.
		if !crate::OPTIONS.windowed {
			graphics::set_fullscreen(quad_ctx, true);
		}
		*/

		MainMenu::new(ctx, quad_ctx).unwrap()
	}

/// Creates a fresh scene stack with the default scene
///
/// Using an `impl` return type is useful for type inference and hides our global state.
pub fn create_stack(ctx: &mut Context, quad_ctx: &mut miniquad::Context) -> impl EventHandler<GameError> {
	let mut stack = SceneStack::new(ctx, GlobalState {});

	stack.push(Box::new(Loading::from(LoadableFn::new(start_main_menu))));

	stack
}
