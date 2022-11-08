use std::ops::DerefMut;

use good_web_game as gwg;
use gwg::audio;
use gwg::cgmath::Point2;
use gwg::goodies::scene::Scene;
use gwg::goodies::scene::SceneSwitch;
use gwg::graphics;
use gwg::graphics::Color;
use gwg::graphics::DrawParam;
use gwg::graphics::PxScale;
use gwg::graphics::Text;
use gwg::graphics::Transform;
use gwg::miniquad::KeyCode;
use gwg::timer;
use logic::generator::Generator;
use logic::generator::PerlinNoise;
use logic::generator::Setting;
use logic::state::Event;
use logic::state::TICKS_PER_SECOND;
use logic::terrain::TileCoord;
use logic::units::BiPolarFraction;
use logic::units::Distance;
use logic::units::Location;
use logic::Input;
use logic::World;
use rand::seq::SliceRandom;
use rand::Rng;

use super::GlobalState;
use crate::assets::asset_batch::image_batch;
use crate::assets::asset_batch::AssetBatch;
use crate::assets::draw_and_clear;
use crate::assets::load_asset_config;
use crate::assets::BuildingBatches;
use crate::assets::ResourceBatches;
use crate::assets::ShipBatches;
use crate::assets::ShipSprites;
use crate::assets::TerrainBatches;
use crate::draw_version;



// #[derive(Debug)] `audio::Source` dose not implement Debug!
pub struct Game {
	terrain_batches: TerrainBatches,
	ship_batches: ShipBatches,
	resource_batches: ResourceBatches,
	building_batches: BuildingBatches,
	sound: audio::Source,
	sound_fishy: audio::Source,
	input_text: String,
	full_screen: bool,
	world: World,
	input: Input,
	meters_per_pixel: f32,
}

impl Game {
	pub fn new(
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::GraphicsContext,
	) -> gwg::GameResult<Self> {
		// TODO: make it configurable or randomize (e.g. use an timestamp),
		//       or implement both.
		let seed: u64 = 44;

		println!(
			"{:.3} [game] loading config...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let render_config = load_asset_config();

		println!(
			"{:.3} [game] loading terrain...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let terrain_batches = TerrainBatches {
			deep: image_batch(ctx, quad_ctx, "img/deepwater0.png")?,
			shallow: image_batch(ctx, quad_ctx, "img/shallowwater.png")?,
			land: image_batch(ctx, quad_ctx, "img/gwg.png")?,
		};

		println!(
			"{:.3} [game] loading ships...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let ship_batches = ShipBatches {
			basic: ShipSprites {
				body: AssetBatch::from_config(ctx, quad_ctx, &render_config, "ship-00")?,
				sail: vec![
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-0")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-1")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-2")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-3")?,
					AssetBatch::from_config(ctx, quad_ctx, &render_config, "sail-00-4")?,
				],
			},
		};

		println!(
			"{:.3} [game] loading resources...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let resource_batches = ResourceBatches {
			fishes: [
				"fish-00", "fish-01", "fish-02", "fish-03", "fish-04", "fish-05", "fish-06",
				"fish-07",
			]
			.into_iter()
			.map(|name| AssetBatch::from_config(ctx, quad_ctx, &render_config, name).unwrap())
			.collect(),
		};

		println!(
			"{:.3} [game] loading buildings...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let building_batches = BuildingBatches {
			harbor: AssetBatch::from_config(ctx, quad_ctx, &render_config, "harbour-00").unwrap(),
		};

		println!(
			"{:.3} [game] loading sounds...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;
		let sound_fishy = audio::Source::new(ctx, "/sound/fischie.ogg")?;


		println!(
			"{:.3} [game] generating world...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);
		// Generate world
		let noise = PerlinNoise;
		let settings = Setting {
			edge_length: 32,
			resource_density: 1.0,
		};

		let mut rng = logic::StdRng::new(0xcafef00dd15ea5e5, seed.into());
		let mut world = noise.generate(&settings, &mut rng);
		world.state.player.vehicle.heading = 1.0;
		world.state.player.vehicle.pos = world.init.terrain.random_passable_location(&mut rng);

		let meters_per_pixel = 30.0 / 1920.0;

		let s = Game {
			terrain_batches,
			ship_batches,
			resource_batches,
			building_batches,
			sound,
			sound_fishy,
			input_text: String::new(),
			full_screen: false,
			world,
			input: Input::default(),
			meters_per_pixel,
		};

		println!(
			"{:.3} [game] ready to go",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);

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

	fn location_to_screen_coords(
		&self,
		ctx: &gwg::Context,
		pos: Location,
	) -> nalgebra::Point2<f32> {
		let screen_coords = gwg::graphics::screen_coordinates(ctx);
		let loc = pos - self.world.state.player.vehicle.pos;
		let sprite_pos = loc.0 / self.meters_per_pixel
			+ logic::glm::vec2(screen_coords.w, screen_coords.h) * 0.5;

		nalgebra::Point2::new(sprite_pos.x, sprite_pos.y)
	}
}

impl Scene<GlobalState> for Game {
	fn name(&self) -> &str {
		"In-Game"
	}

	fn update(
		&mut self,
		glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
	) -> SceneSwitch<GlobalState> {
		use gwg::input::keyboard::is_key_pressed;

		while gwg::timer::check_update_time(ctx, TICKS_PER_SECOND.into()) {
			let mut rudder = 0.0;

			// Rudder input
			if is_key_pressed(ctx, KeyCode::Left) {
				rudder -= 1.0;
			}
			if is_key_pressed(ctx, KeyCode::Right) {
				rudder += 1.0;
			}

			self.input.rudder = BiPolarFraction::from_f32(rudder).unwrap();
			let events = self.world.state.update(&self.world.init, &self.input);

			for ev in events {
				match ev {
					Event::Fishy => self.sound_fishy.play(ctx).unwrap(),
				}
			}
		}

		if is_key_pressed(&ctx, KeyCode::Escape) {
			SceneSwitch::Pop
		} else {
			SceneSwitch::None
		}
	}

	fn draw(
		&mut self,
		glob: &mut GlobalState,
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

			let lt = TileCoord::try_from((player_pos - dst).max(Location::ORIGIN)).expect("no lt");
			let rb = TileCoord::try_from(player_pos + dst).expect("no rb");

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
					// if TileCoord::from(self.world.state.player.vehicle.pos) == tc {
					// 	continue;
					// }


					let scale = logic::TILE_SIZE as f32 / self.meters_per_pixel / 256.0;
					let loc = tc.to_location().0; // - logic::glm::vec1(logic::TILE_SIZE as f32).xx() * 0.5;
					let param = DrawParam::new()
						.dest(self.location_to_screen_coords(ctx, Location(loc)))
						.scale(logic::glm::vec2(scale, scale));

					let batch = {
						if tile.0 < -5 {
							&mut self.terrain_batches.deep
						} else if tile.0 < 0 {
							&mut self.terrain_batches.shallow
						} else {
							&mut self.terrain_batches.land
						}
					};

					batch.add(param);
				}
			}
		}

		let ship_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE
				/ self.meters_per_pixel
				/ self.ship_batches.basic.body.params().width as f32,
		)
		.xx();
		let ship_pos = self.world.state.player.vehicle.pos.0
			- logic::glm::vec1(2.5 * logic::VEHICLE_SIZE).xx() * 0.5;
		let param = DrawParam::new()
			.dest(self.location_to_screen_coords(ctx, Location(ship_pos)))
			.scale(ship_scale);
		let heading = f64::from(self.world.state.player.vehicle.heading);
		let ship_heading = -heading + std::f64::consts::PI;
		self.ship_batches.basic.body.add_frame(
			0.0,
			ship_heading,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			param,
		);

		let max_sail = self.ship_batches.basic.sail.len() - 1;
		let sail_reefing = match self.world.state.player.vehicle.sail.reefing {
			logic::state::Reefing::Reefed(n) => n,
		};

		let sail_ass = &mut self.ship_batches.basic.sail[usize::from(sail_reefing).min(max_sail)];
		let sail_scale = logic::glm::vec1(
			2.5 * logic::VEHICLE_SIZE / self.meters_per_pixel / sail_ass.params().width as f32,
		)
		.xx();
		let sail_param = DrawParam::new()
			.dest(self.location_to_screen_coords(ctx, Location(ship_pos)))
			.scale(sail_scale);
		let orientation = f64::from(self.world.state.player.vehicle.sail.orientation);
		let sail_orient = -orientation + std::f64::consts::PI;

		let sail_ass = &mut self.ship_batches.basic.sail[usize::from(sail_reefing).min(max_sail)];
		sail_ass.add_frame(
			// We need the sail orientation, minus the heading (because the model is in a rotating frame), plus a half turn (because the model is half way turned around).
			sail_orient - ship_heading + std::f64::consts::PI,
			ship_heading,
			f64::from(self.world.state.player.vehicle.angle_of_list),
			sail_param,
		);

		for resource in &self.world.state.resources {
			let mut rng = logic::StdRng::new(
				(resource.loc.0.x * 100.0) as u128,
				(resource.loc.0.y * 100.0) as u128,
			);

			let resource_pos =
				resource.loc.0 - logic::glm::vec1(logic::RESOURCE_PACK_FISH_SIZE).xx() * 0.5;
			let dest = self.location_to_screen_coords(ctx, Location(resource_pos));

			let batch = self.resource_batches.fishes.choose_mut(&mut rng).unwrap();

			let resource_scale = logic::glm::vec1(
				logic::RESOURCE_PACK_FISH_SIZE
					/ self.meters_per_pixel
					/ batch.params().width as f32,
			)
			.xx();
			let param = DrawParam::new().dest(dest).scale(resource_scale);

			batch.add_frame(0.0, rng.gen::<f64>() * std::f64::consts::TAU, 0.0, param);
		}

		for harbor in &self.world.state.harbors {
			let harbor_scale = logic::glm::vec1(
				logic::HARBOR_SIZE
					/ self.meters_per_pixel
					/ self.building_batches.harbor.params().width as f32,
			)
			.xx();
			let harbor_pos =
				harbor.loc.0 - logic::glm::vec1(logic::RESOURCE_PACK_FISH_SIZE).xx() * 0.5;
			let param = DrawParam::new()
				.dest(self.location_to_screen_coords(ctx, Location(harbor_pos)))
				.scale(harbor_scale);

			self.building_batches
				.harbor
				.add_frame(0.0, f64::from(harbor.orientation), 0.0, param);
		}

		draw_and_clear(
			ctx,
			quad_ctx,
			[
				&mut self.terrain_batches.deep,
				&mut self.terrain_batches.shallow,
				&mut self.terrain_batches.land,
			]
			.into_iter()
			.chain(
				self.resource_batches
					.fishes
					.iter_mut()
					.map(DerefMut::deref_mut),
			)
			.chain([
				self.building_batches.harbor.deref_mut(),
				self.ship_batches.basic.body.deref_mut(),
			])
			.chain(
				self.ship_batches
					.basic
					.sail
					.iter_mut()
					.map(DerefMut::deref_mut),
			),
		)?;

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

		// Current Wind
		let input_text = Text::new(format!(
			"Wind: {:.2} bf, {:.0}°",
			self.world.state.wind.magnitude(),
			self.world.state.wind.angle().to_degrees(),
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 40.0), Color::WHITE),
			Color::BLACK,
		)?;
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&Text::new("→"),
			(
				Point2::new(70.0, 40.0),
				self.world.state.wind.angle(),
				Color::WHITE,
			),
			Color::BLACK,
		)?;

		// Current Ship states
		let input_text = Text::new(format!(
			"Ship: {:.2} m/s, fish {:.0} kg",
			self.world.state.player.vehicle.ground_speed(),
			self.world.state.player.vehicle.fish.0
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 60.0), Color::WHITE),
			Color::BLACK,
		)?;

		// Current Ship states
		let input_text = Text::new(format!(
			"Ori: {:.2}, List: {:.0}°",
			self.world
				.state
				.player
				.vehicle
				.heading
				.rem_euclid(std::f32::consts::TAU),
			self.world.state.player.vehicle.angle_of_list.to_degrees()
		));
		self.draw_text_with_halo(
			ctx,
			quad_ctx,
			&input_text,
			(Point2::new(100.0, 80.0), Color::WHITE),
			Color::BLACK,
		)?;


		let mut text = Text::new("Press 'A' for a sound, and Enter to clear");
		text.set_font(Default::default(), PxScale::from(32.));
		graphics::draw(ctx, quad_ctx, &text, (Point2::new(0.0, 300.), Color::BLACK))?;
		let mut text = Text::new(format!("»{}«", self.input_text.as_str()));
		text.set_font(Default::default(), PxScale::from(64.));
		graphics::draw(ctx, quad_ctx, &text, (Point2::new(0.0, 350.), Color::BLACK))?;

		// Print version info
		draw_version(ctx, quad_ctx)?;

		gwg::graphics::present(ctx, quad_ctx)?;

		Ok(())
	}

	fn key_down_event(
		&mut self,
		glob: &mut GlobalState,
		ctx: &mut gwg::Context,
		quad_ctx: &mut gwg::miniquad::Context,
		keycode: gwg::miniquad::KeyCode,
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
			// TODO: limit reefing
			self.input.reefing = self.input.reefing.increase();
		} else if keycode == KeyCode::Down {
			self.input.reefing = self.input.reefing.decrease();
		}

		if keycode == KeyCode::F11 {
			self.full_screen = !self.full_screen;
			println!("{}", self.full_screen);
			good_web_game::graphics::set_fullscreen(quad_ctx, self.full_screen);
			good_web_game::graphics::set_drawable_size(quad_ctx, 600, 480);
		}
	}

	/*
	TODO: what to do about that?
	fn text_input_event(
		&mut self,
		_ctx: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::Context,
		character: char,
	) {
		self.input_text.push(character);
	}
	*/

	fn resize_event(
		&mut self,
		glob: &mut GlobalState,
		context: &mut gwg::Context,
		_quad_ctx: &mut gwg::miniquad::GraphicsContext,
		w: f32,
		h: f32,
	) {
		let coordinates = graphics::Rect::new(0., 0., w, h);

		graphics::set_screen_coordinates(context, coordinates).expect("Can't resize the window");
	}
}
