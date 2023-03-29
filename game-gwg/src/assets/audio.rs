use good_web_game as gwg;
use good_web_game::audio;
use gwg::GameResult;


// #[derive(Debug)] `audio::Source` dose not implement Debug!
pub struct Audios {
	pub sound_enabled: bool,
	pub music_enabled: bool,
	pub sound: audio::Source,
	pub fail_sound: audio::Source,
	pub sell_sound: audio::Source,
	pub upgrade_sound: audio::Source,
	pub sound_fishy_1: audio::Source,
	pub sound_fishy_2: audio::Source,
	pub sound_fishy_3: audio::Source,
	pub sound_shoe: audio::Source,
	pub sound_blub: audio::Source,
	pub sound_grass: audio::Source,
	pub collision_harbor: audio::Source,
	pub collision_beach: audio::Source,
	pub music_0: audio::Source,
	pub water_sound_0: audio::Source,
	pub water_sound_1: audio::Source,
	/// Indicates whether there was a harbor collision in the last frame
	pub collision_harbor_in_this_frame: bool,
	/// Indicates whether there was a beach collision in the last frame
	pub collision_beach_in_this_frame: bool,
}
impl Audios {
	pub fn load(ctx: &mut gwg::Context) -> GameResult<Self> {
		println!(
			"{:.3} [audio] loading music...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);

		let mut music_0 = audio::Source::new(ctx, "/music/sailing-chanty.ogg")?;
		music_0.set_repeat(true);
		music_0.set_volume(ctx, 0.7)?;

		println!(
			"{:.3} [audio] loading sounds...",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);

		let sound = audio::Source::new(ctx, "/sound/pew.ogg")?;
		let fail_sound = audio::Source::new(ctx, "/sound/invalid.ogg")?;
		let upgrade_sound = audio::Source::new(ctx, "/sound/upgrade.ogg")?;
		let sound_fishy_1 = audio::Source::new(ctx, "/sound/fischie.ogg")?;
		let sound_fishy_2 = audio::Source::new(ctx, "/sound/fischie2.ogg")?;
		let sound_fishy_3 = audio::Source::new(ctx, "/sound/fischie3.ogg")?;
		let sound_shoe = audio::Source::new(ctx, "/sound/shoe.ogg")?;
		let sound_blub = audio::Source::new(ctx, "/sound/blub.ogg")?;
		let sound_grass = audio::Source::new(ctx, "/sound/grass.ogg")?;
		let collision_harbor = audio::Source::new(ctx, "/sound/harbor_collision.ogg")?;
		let collision_beach = audio::Source::new(ctx, "/sound/sand_collision.ogg")?;

		let mut sell_sound = audio::Source::new(ctx, "/sound/sell-sound.ogg")?;
		sell_sound.set_repeat(true);
		sell_sound.set_volume(ctx, 0.)?;
		let mut water_sound_0 = audio::Source::new(ctx, "/sound/waterssoftloop.ogg")?;
		water_sound_0.set_repeat(true);
		let mut water_sound_1 = audio::Source::new(ctx, "/sound/waterstrongloop.ogg")?;
		water_sound_1.set_repeat(true);
		water_sound_1.set_volume(ctx, 0.)?;

		println!(
			"{:.3} [audio] all audios loaded",
			gwg::timer::time_since_start(ctx).as_secs_f64()
		);

		Ok(Audios {
			sound_enabled: false,
			music_enabled: false,
			sound,
			fail_sound,
			sell_sound,
			upgrade_sound,
			sound_fishy_1,
			sound_fishy_2,
			sound_fishy_3,
			sound_shoe,
			sound_blub,
			sound_grass,
			collision_harbor,
			collision_beach,
			music_0,
			water_sound_0,
			water_sound_1,
			collision_harbor_in_this_frame: false,
			collision_beach_in_this_frame: false,
		})
	}

	/// Enables or disables background music
	pub fn enable_music(&mut self, ctx: &mut gwg::Context, enabled: bool) -> gwg::GameResult {
		if self.music_enabled == enabled {
			// Done
		} else {
			self.music_enabled = enabled;
			if enabled {
				// Actually enable sounds
				self.music_0.play(ctx)?;
			} else {
				// Disable sounds
				self.music_0.stop(ctx)?;
			}
		}

		Ok(())
	}

	/// Enables or disables sound effects
	pub fn enable_sound(&mut self, ctx: &mut gwg::Context, enabled: bool) -> gwg::GameResult {
		if self.sound_enabled == enabled {
			// Done
		} else {
			self.sound_enabled = enabled;
			if enabled {
				// Actually enable sounds
				self.water_sound_0.play(ctx)?;
				self.water_sound_1.play(ctx)?;
				self.sell_sound.play(ctx)?;
			} else {
				// Disable sounds
				self.water_sound_0.stop(ctx)?;
				self.water_sound_1.stop(ctx)?;
				self.sell_sound.stop(ctx)?;

				// Also disable event sound
				self.sound_fishy_1.stop(ctx)?;
				self.sound_fishy_2.stop(ctx)?;
				self.sound_fishy_3.stop(ctx)?;
				self.sound_shoe.stop(ctx)?;
				self.sound_blub.stop(ctx)?;
				self.sound_grass.stop(ctx)?;
			}
		}

		Ok(())
	}
}
