mod in_game;
mod loading;


use good_web_game::event::EventHandler;
use good_web_game::event::{self,};
use good_web_game::goodies::scene::SceneStack;
use good_web_game::graphics;
use good_web_game::Context;
use good_web_game::GameError;
pub use in_game::Game;

use self::loading::LoadableFn;
use self::loading::Loading;


/// Some global state (between the scenes)
struct GlobalState {}


/// Creates a fresh scene stack with the default scene
///
/// Using an `impl` return type is useful for type inference and hides our global state.
pub fn create_stack(ctx: &mut Context) -> impl EventHandler<GameError> {
	let mut stack = SceneStack::new(ctx, GlobalState {});

	fn ng(_: &mut GlobalState, ctx: &mut Context, quad_ctx: &mut event::GraphicsContext) -> Game {
		// Set Full screen mode again, if requested, to correctly apply it.
		// Appears buggy if not done here again.
		if !crate::OPTIONS.windowed {
			graphics::set_fullscreen(quad_ctx, true);
		}

		Game::new(ctx, quad_ctx).unwrap()
	}

	stack.push(Box::new(Loading::from(LoadableFn::new(ng))));

	stack
}
