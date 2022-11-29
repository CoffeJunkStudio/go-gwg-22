use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::fmt;

use enum_map::Enum;
use nalgebra_glm::vec2;
use nalgebra_glm::Vec2;
use rand::distributions::Distribution;
use rand::Rng;
use rand_distr::Beta;
use serde::Deserialize;
use serde::Serialize;

use crate::terrain::TileCoord;
use crate::units::BiPolarFraction;
use crate::units::Fraction;
use crate::units::Location;
use crate::units::Tick;
use crate::units::Wind;
use crate::Input;
use crate::ResourcePack;
use crate::ResourcePackContent;
use crate::StdRng;
use crate::WorldInit;
use crate::FRICTION_CROSS_SPEED_FACTOR;
use crate::FRICTION_GROUND_SPEED_FACTOR;
use crate::HARBOR_DOCKING_SPEED;
use crate::HARBOR_EFFECT_SIZE;
use crate::HARBOR_MAX_SPEED;
use crate::HARBOR_SIZE;
use crate::MAX_TRACTION;
use crate::MAX_WIND_SPEED;
use crate::RESOURCE_PACK_FISH_SIZE;
use crate::TICKS_PER_SECOND;
use crate::VEHICLE_DEADWEIGHT;
use crate::VEHICLE_SIZE;
use crate::WIND_CHANGE_INTERVAL;



const DELTA: f32 = 1_f32 / TICKS_PER_SECOND as f32;


/// Normalize an angle in positive range [0,2π)
fn normalize_angle_pos(angle: f32) -> f32 {
	angle.rem_euclid(TAU)
}

/// Normalize an angle in range [-π,π)
fn normalize_angle_rel(angle: f32) -> f32 {
	let pos = normalize_angle_pos(angle);
	if pos > PI {
		pos - TAU
	} else {
		pos
	}
}


/// Events that can happen between ticks
#[derive(Debug, Clone)]
pub enum Event {
	Fishy,
	Starfish,
	Shoe,
	Grass,
	TileCollision(f32),
	HarborCollision(f32),
}



/// The dynamic part of the world
#[derive(Debug, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct WorldState {
	/// The point in time of this state
	pub timestamp: Tick,
	/// The active player
	pub player: Player,
	/// The full list of collectables on the map
	pub resources: Vec<ResourcePack>,
	/// The full list of harbors
	pub harbors: Vec<Harbor>,
	/// The currently prevailing wind condition
	pub wind: Wind,
}

impl WorldState {
	pub fn update(&mut self, init: &WorldInit, inputs: &Input) -> Vec<Event> {
		let mut events = Vec::new();

		// Increment timestamp
		self.timestamp = self.timestamp.next();

		// Apply user inputs
		self.player.vehicle.apply_input(*inputs);

		// Update fishies
		for r in &mut self.resources {
			r.update(self.timestamp);
		}

		// Update wind
		self.wind = {
			if init.dbg.wind_turning {
				// Turning wind
				Wind::from_polar(
					(self.timestamp.0
						% (u64::from(TICKS_PER_SECOND) * u64::from(WIND_CHANGE_INTERVAL))) as f32
						/ (u64::from(TICKS_PER_SECOND) * u64::from(WIND_CHANGE_INTERVAL)) as f32
						* std::f32::consts::TAU,
					MAX_WIND_SPEED,
				)
			} else if let Some(dir) = init.dbg.fixed_wind_direction {
				// Fixed wind
				Wind::from_polar(dir, MAX_WIND_SPEED)
			} else {
				// Normal randomized wind

				// Using a beta distribution with α=5, β=2 for the Magnitude
				let beta = Beta::new(5.0, 2.0).unwrap();

				let interval = u64::from(TICKS_PER_SECOND) * u64::from(WIND_CHANGE_INTERVAL);
				let earlier = self.timestamp.0 / interval;
				let later = earlier + 1;
				let offset = self.timestamp.0 - earlier * interval;

				let early = {
					let mut rng = StdRng::new(
						0xcafef00dd15ea5e5,
						0xa02bdbf7bb3c0a7ac28fa16a64abf96
							^ u128::from(init.seed) ^ u128::from(earlier),
					);

					let angle = rng.gen::<f32>() * std::f32::consts::TAU;
					let magnitude = beta.sample(&mut rng) * MAX_WIND_SPEED;
					Wind::from_polar(angle, magnitude)
				};
				let late = {
					let mut rng = StdRng::new(
						0xcafef00dd15ea5e5,
						0xa02bdbf7bb3c0a7ac28fa16a64abf96
							^ u128::from(init.seed) ^ u128::from(later),
					);

					let angle = rng.gen::<f32>() * std::f32::consts::TAU;
					let magnitude = beta.sample(&mut rng) * MAX_WIND_SPEED;
					Wind::from_polar(angle, magnitude)
				};

				let lerpy = nalgebra_glm::lerp(&early.0, &late.0, offset as f32 / interval as f32);
				Wind(lerpy)
			}
		};

		//let water_consumption = crate::WATER_CONSUMPTION * DELTA;

		{
			let p = &mut self.player;

			// in s
			let duration = DELTA;

			// Speed cheat
			if init.dbg.ship_engine {
				let speed_per_sail_area = 1. / 20.;
				let sail_area = p.vehicle.sail.sail_area();
				let speed = sail_area * speed_per_sail_area;

				if p.vehicle.velocity.norm() < speed {
					let tang_speed = p.vehicle.velocity.dot(&p.vehicle.tangent_vec());
					let head_speed = p.vehicle.velocity.dot(&p.vehicle.heading_vec());

					let diff_speed = (speed.powi(2) - tang_speed.powi(2)).sqrt() - head_speed;

					p.vehicle.velocity += p.vehicle.heading_vec() * diff_speed;
				}
			}


			// in m/s²
			let acceleration = {
				let true_wind = self.wind.0;
				let apparent_wind = true_wind - p.vehicle.velocity;
				let ship_angle = p.vehicle.heading;

				let local_wind_angle = {
					let diff = f32::atan2(apparent_wind.y, apparent_wind.x) - ship_angle;

					// Normalized to [-π, π)
					normalize_angle_rel(diff)
				};

				let local_triangle_sail_angle =
					normalize_angle_rel(local_wind_angle + PI).clamp(-PI / 2., PI / 2.) - PI;
				p.vehicle.sail.orientation_triangle = local_triangle_sail_angle + ship_angle;
				let local_square_sail_angle =
					normalize_angle_rel(local_wind_angle).clamp(-PI / 2., PI / 2.);
				p.vehicle.sail.orientation_rectangle = local_square_sail_angle + ship_angle;


				let sail_drag_ness = 1.
					- p.vehicle
						.sail
						.orientation_triangle_vec()
						.dot(&apparent_wind.normalize())
						.abs();

				let sail_drag = apparent_wind * sail_drag_ness;


				let static_ship_area = 1.;
				let sail_area = p.vehicle.sail.sail_area();

				let prop = sail_drag * sail_area + apparent_wind * static_ship_area;

				let direction = apparent_wind.normalize();

				// in W
				let power = prop.magnitude();
				// in J
				let work = power * duration;

				// Acceleration

				// in m/s
				let speed = p.vehicle.ground_speed();
				// in kg
				let mass = p.vehicle.mass();

				// in m/s²
				let acceleration = (-speed + (speed * speed + 2.0 * work / mass).sqrt()) / duration;

				direction * acceleration
			};

			/* debugging
			println!(
				"{:4.4} ({:1.1}) +- {:4.4} / {:4.4}",
				p.vehicle.speed,
				p.vehicle.engine.throttle.to_f32(),
				acceleration,
				p.vehicle.friction_deacceleration()
			);
			*/

			let friction = p.vehicle.friction_deacceleration();


			let vel_0 = p.vehicle.velocity;

			let acc = acceleration + friction;

			// Save the old tile and position
			let old_tile: TileCoord = p.vehicle.pos.try_into().expect("Player is out of bounds");
			let old_pos = p.vehicle.pos.0;
			let old_velo = p.vehicle.velocity;

			// Move according to acceleration & velocity
			p.vehicle.velocity += acc * duration;
			let distance = duration * (vel_0 + duration * acc);
			p.vehicle.pos.0 += distance;

			// Keep the player on the Torus-world
			p.vehicle.pos = init.terrain.map_loc_on_torus(p.vehicle.pos);

			// Terrain interaction
			// First check whether the player is still on the map, and if so
			// retrieve its new tile.
			if let Ok(new_tile) = TileCoord::try_from(p.vehicle.pos) {
				// Only check collisions if the player is in passable water.
				// So the player is free to move around if he glitched into terrain, to get out
				if Some(true) == init.terrain.try_get(old_tile).map(|t| t.is_passable()) {
					// Check if the player tries to go into impassable terrain
					if Some(true) != init.terrain.try_get(new_tile).map(|t| t.is_passable()) {
						// TODO: maybe we want to handle this differently
						// Ship bounce off land
						p.vehicle.pos.0 = old_pos;

						p.vehicle.velocity *= -0.5;

						if old_tile.x == new_tile.x {
							// restore x component sign
							p.vehicle.velocity.x *= -1.;
						}
						if old_tile.y == new_tile.y {
							// restore y component sign
							p.vehicle.velocity.y *= -1.;
						}

						// Add event about collision
						events.push(Event::TileCollision(old_velo.norm()));
					}
				}
			} else {
				// Player off map
				// Can not happen in Torus-world!
				eprintln!("Player pos: {:?}", p.vehicle.pos);
				panic!("Player went off the Torus!")

				// Clamp
				//p.vehicle.pos.0 -= distance;
				//p.vehicle.velocity = Vec2::new(0., 0.);
			}

			// Harbor collision
			for harbor in &self.harbors {
				let coll_dist = (HARBOR_SIZE + VEHICLE_SIZE) * 0.5;
				let distance = init.terrain.torus_distance(p.vehicle.pos,harbor.loc).0.norm();
				let old_distance = init.terrain.torus_distance(Location(old_pos),harbor.loc).0.norm();
				// Only check if the player isn't inside yet
				if old_distance >= coll_dist {
					// Check if the player went inside
					if distance < coll_dist {
						// Reset player pos
						p.vehicle.pos.0 = old_pos;

						// Bounce off away from the harbor
						let head = (old_pos - harbor.loc.0).normalize();
						//let turn = Rotation2::new(PI / 2.);
						//let tang = turn * head;

						let head_speed = p.vehicle.velocity.dot(&head);
						p.vehicle.velocity -= head * head_speed * 1.5;

						// Add event about collision
						events.push(Event::HarborCollision(old_velo.norm()));
					}
				}
				// Make a ship docked, if within harbor range, without a sail, slow enough
				if distance < HARBOR_EFFECT_SIZE
					&& p.vehicle.sail.reefing == Reefing(0)
					&& p.vehicle.velocity.norm() <= HARBOR_DOCKING_SPEED
				{
					// Dock the ship
					p.vehicle.velocity = vec2(0., 0.);
				}
			}

			/* TODO: how about a shore-based breaking
			 * Tho we would need a (too) shallow water visualization
			// Apply breaking
			let wheel_speed = p.vehicle.wheel_speed();
			let breaking_impulse = p.vehicle.engine.breaking.to_f32() * BREAKING_DEACCL * DELTA;
			let breaking_impulse = breaking_impulse.min(wheel_speed.abs());
			p.vehicle.velocity -= breaking_impulse * wheel_speed.signum() * p.vehicle.heading_vec();
			*/


			// Apply steering

			// distance traveled by rolling wheels
			let distance_norm = distance.dot(&p.vehicle.heading_vec());
			// steering angle relative to the current roll direction (i.e. relative to the heading)
			let steering_angle = p.vehicle.ruder.to_f32().abs() * crate::VEHICLE_MAX_STEERING_ANGLE;
			let turning_circle_radius = crate::VEHICLE_WHEEL_BASE / steering_angle.sin();

			// Turning angle
			let angle = distance_norm / turning_circle_radius;

			let angle = angle.max(0.02);

			if p.vehicle.ruder.to_f32().abs() > 0.01 {
				p.vehicle.heading += angle * p.vehicle.ruder.to_f32().signum();
			}

			// Turning by traction

			let head_speed = p.vehicle.wheel_speed();
			let cross_speed = p.vehicle.cross_speed() * 0.5;

			p.vehicle.angle_of_list = (-(cross_speed / MAX_TRACTION / 2.) * PI).clamp(-PI, PI);

			let cross_traction_speed = cross_speed.clamp(-MAX_TRACTION, MAX_TRACTION);

			let head_velo = head_speed.signum()
				* f32::sqrt(head_speed.powi(2) + cross_traction_speed.powi(2))
				* p.vehicle.heading_vec();
			let cross_velo = cross_speed.signum()
				* f32::sqrt(cross_speed.powi(2) - cross_traction_speed.powi(2))
				* p.vehicle.tangent_vec();

			p.vehicle.velocity = head_velo + cross_velo;
		}

		let WorldState {
			player,
			resources,
			..
		} = self;

		// Process resource collection
		{
			let p = player;

			resources.retain(|r| {
				let dist = VEHICLE_SIZE / 2. + RESOURCE_PACK_FISH_SIZE / 2.;
				let tor_dist = init.terrain.torus_distance(r.loc, p.vehicle.pos);

				if tor_dist.0.norm() < dist {
					// Store the fish in the ship
					p.vehicle.resource_weight += r.content.weight;
					p.vehicle.resource_value += r.content.value;

					// Emit event for sound effects
					{
						use ResourcePackContent::*;
						match r.content {
							Fish0 | Fish1 | Fish2 | Fish3 | Fish4 | Fish5 | Fish6 | Fish7 => {
								events.push(Event::Fishy)
							},
							Starfish0 | Starfish1 | Starfish2 | Starfish3 | Starfish4 => {
								events.push(Event::Starfish);
							},
							Shoe0 | Shoe1 => {
								events.push(Event::Shoe);
							},
							Grass0 | Grass1 => {
								events.push(Event::Grass);
							},
						}
					}

					// Let the fish be removed from the world
					false
				} else {
					true
				}
			});
		}

		events
	}

	/// Get options for trading
	pub fn get_trading(&mut self, init: &WorldInit) -> Option<TradeOption> {
		let mut min_dist_n_idx: Option<(f32, usize)> = None;
		for (idx, h) in self.harbors.iter().enumerate() {
			let dist = init.terrain.torus_distance(self.player.vehicle.pos,h.loc).0.norm();
			if dist < HARBOR_EFFECT_SIZE {
				match min_dist_n_idx {
					None => {
						min_dist_n_idx = Some((dist, idx));
					},
					Some((d, _)) if dist < d => {
						min_dist_n_idx = Some((dist, idx));
					},
					_ => {},
				}
			}
		}

		min_dist_n_idx
			.map(|(_d, idx)| idx)
			.map(|idx| TradeOption::new(self, idx))
	}
}

/// Represents a trading option
///
///
pub struct TradeOption<'a> {
	/// The world state
	state: &'a mut WorldState,
	/// The harbor in question
	///
	/// This is an index into the `harbors` field on the above `state`.
	harbor_idx: usize,
	/// Base price for fish, in money
	base_price: u64,
	/// Amount of fish traded so far, in kg
	traded_fish_amount: u32,
}
impl<'a> TradeOption<'a> {
	fn new(state: &'a mut WorldState, harbor_idx: usize) -> Self {
		Self {
			state,
			harbor_idx,
			base_price: 1,
			traded_fish_amount: 0,
		}
	}
}

impl TradeOption<'_> {
	/// The harbor with which trade is possible
	pub fn get_harbor(&mut self) -> &mut Harbor {
		&mut self.state.harbors[self.harbor_idx]
	}

	/// The the current offered price for fish, in money
	pub fn get_price_for_fish(&self) -> u64 {
		self.base_price
	}

	/// Returns the price for upgrading the sail to the next level (if any)
	///
	/// Returns `None` if already at max level
	pub fn get_price_for_sail_upgrade(&self) -> Option<u64> {
		self.state
			.player
			.vehicle
			.sail
			.kind
			.upgrade()
			.map(|s| s.value())
	}

	/// Returns the price for upgrading the sail to the next level (if any)
	///
	/// Returns `None` if already at max level
	pub fn get_price_of_hull_upgrade(&self) -> Option<u64> {
		self.state.player.vehicle.hull.upgrade().map(|s| s.value())
	}

	/// Try to upgrade the sail to the next level (if any)
	///
	/// This function, if successful, will advance the ships sail level, and
	/// reduce the players money accordingly.
	///
	/// Returns `Ok` if successful.
	pub fn upgrade_sail(&mut self) -> Result<(), UpgradeError> {
		// Do not trade if the player is too fast
		if !self.has_player_valid_speed() {
			// Player not docked
			return Err(UpgradeError::NotDocked);
		}

		let sail = &mut self.state.player.vehicle.sail.kind;
		let upgrade_opt = sail.upgrade();

		if let Some(upgrade) = upgrade_opt {
			let upgrade_cost = upgrade.value();

			let money = &mut self.state.player.money;
			if *money >= upgrade_cost {
				*money -= upgrade_cost;
				*sail = upgrade;

				Ok(())
			} else {
				// Insufficient funds
				Err(UpgradeError::InsufficientFunds)
			}
		} else {
			// Already at max level
			Err(UpgradeError::MaxLevel)
		}
	}

	/// Try to upgrade the hull to the next level (if any)
	///
	/// This function, if successful, will advance the ships hull level, and
	/// reduce the players money accordingly.
	///
	/// Returns `Ok` if successful.
	pub fn upgrade_hull(&mut self) -> Result<(), UpgradeError> {
		// Do not trade if the player is too fast
		if !self.has_player_valid_speed() {
			// Player not docked
			return Err(UpgradeError::NotDocked);
		}

		let hull = &mut self.state.player.vehicle.hull;
		let upgrade_opt = hull.upgrade();

		if let Some(upgrade) = upgrade_opt {
			let upgrade_cost = upgrade.value();

			let money = &mut self.state.player.money;
			if *money >= upgrade_cost {
				*money -= upgrade_cost;
				*hull = upgrade;

				Ok(())
			} else {
				// Insufficient funds
				Err(UpgradeError::InsufficientFunds)
			}
		} else {
			// Already at max level
			Err(UpgradeError::MaxLevel)
		}
	}

	/// The monetary volume traded so far, in money
	pub fn get_traded_volume(&self) -> u64 {
		u64::from(self.traded_fish_amount) * self.base_price
	}

	/// Check whether the player has a proper speed for trading
	pub fn has_player_valid_speed(&self) -> bool {
		self.state.player.vehicle.ground_speed() <= HARBOR_MAX_SPEED
	}

	/// Returns the amount of fish the player has left
	pub fn players_fish_amount(&self) -> u32 {
		self.state.player.vehicle.resource_weight
	}

	/// Sell `amount` (in kg) of fish, returns the proceeds
	pub fn sell_fish(&mut self, amount: u32) -> Option<u32> {
		// Do not trade if the player is too fast
		if !self.has_player_valid_speed() {
			return None;
		}

		// Find the actual amount sellable
		let (weight, value) = {
			if amount >= self.state.player.vehicle.resource_weight {
				(
					self.state.player.vehicle.resource_weight,
					self.state.player.vehicle.resource_value,
				)
			} else {
				(
					amount,
					u64::from(amount) * self.state.player.vehicle.resource_value
						/ u64::from(self.state.player.vehicle.resource_weight),
				)
			}
		};

		// Calculate the generated proceeds
		let proceeds = value * self.base_price;

		// Remove the fish from the player
		// This must not underflow, because we checked above
		self.state.player.vehicle.resource_weight -= weight;
		self.state.player.vehicle.resource_value -= value;

		// Deposit proceeds into the player's account
		// If the player manages to get 2^64 money, we just keep it that way
		self.state.player.money = self.state.player.money.saturating_add(proceeds);

		// Remember the session trade volume
		self.traded_fish_amount += weight;

		Some(weight)
	}
}


/// Represents the reason for the failure of upgrading gear
#[derive(Debug, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub enum UpgradeError {
	NotDocked,
	InsufficientFunds,
	MaxLevel,
}
impl fmt::Display for UpgradeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let msg = match self {
			Self::NotDocked => "Not docked at harbor",
			Self::InsufficientFunds => "Insufficient funds",
			Self::MaxLevel => "Already at max sail level",
		};
		write!(f, "{}", msg)
	}
}

/// Represents the car of a player
#[derive(Debug, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Harbor {
	/// Absolute position in meters
	pub loc: Location,
	/// Orientation in radians, zero is world x
	pub orientation: f32,
}


/// Represents the car of a player
#[derive(Debug, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Vehicle {
	/// The ship hull type
	pub hull: ShipHull,
	/// Absolute position in meters
	pub pos: Location,
	/// Current movement in m/s
	///
	/// Notice that this direction might differ from the
	/// `heading` if drifting, or it could be anti-parallel
	/// if driving in reverse.
	pub velocity: Vec2,
	/// Current heading in radians, zero is world x
	pub heading: f32,
	/// Current angle of list in radians, zero in upright
	///
	/// A negative values means a tilt to the left, positive values tilt to the right.
	pub angle_of_list: f32,
	/// Gives the current steering.
	///
	/// Steering is always relative to `heading`.
	///
	/// See [Input::rudder]
	pub ruder: BiPolarFraction,
	/// State of the engine
	pub sail: Sail,
	//// Amount of fish and stuff on board in kg
	pub resource_weight: u32,
	//// Amount of fish and stuff on board in money
	pub resource_value: u64,
}
impl Vehicle {
	/// Ground speed in m/s
	///
	/// This is simply the magnitude of `velocity`.
	pub fn ground_speed(&self) -> f32 {
		self.velocity.magnitude()
	}

	/// Heading as unit vector.
	pub fn heading_vec(&self) -> Vec2 {
		Vec2::new(self.heading.cos(), self.heading.sin())
	}

	/// Tangent vector which is orthogonal to heading.
	pub fn tangent_vec(&self) -> Vec2 {
		let tangent = self.heading + core::f32::consts::FRAC_PI_2;
		Vec2::new(tangent.cos(), tangent.sin())
	}

	/// The speed covered by the wheels.
	///
	/// Notice this gives the "signed" speed in the direction of `heading`.
	pub fn wheel_speed(&self) -> f32 {
		self.velocity.dot(&self.heading_vec())
	}

	/// The speed orthogonal to the wheels
	///
	/// Notice this is the "signed" speed in the direction of the tangent (i.e. the orthogonal of `heading`).
	pub fn cross_speed(&self) -> f32 {
		self.velocity.dot(&self.tangent_vec())
	}

	/// The acceleration caused by friction in m/s
	///
	/// This acceleration is vectorial thus it can be just added to the `velocity`.
	pub fn friction_deacceleration(&self) -> Vec2 {
		let rolling_friction =
			-self.wheel_speed() * FRICTION_GROUND_SPEED_FACTOR * self.heading_vec();

		let sliding_friction =
			-self.cross_speed() * FRICTION_CROSS_SPEED_FACTOR * self.tangent_vec();

		rolling_friction + sliding_friction
	}

	/// Apply the given `input` to this vehicle
	pub fn apply_input(&mut self, input: Input) {
		Input {
			reefing: self.sail.reefing,
			rudder: self.ruder,
		} = input;
	}

	/// Returns the total mass of the vehicle (inclusive payloads) in kilogram
	pub fn mass(&self) -> f32 {
		VEHICLE_DEADWEIGHT + self.resource_weight as f32
	}
}

impl Default for Vehicle {
	fn default() -> Self {
		Self {
			hull: Default::default(),
			pos: Default::default(),
			sail: Default::default(),
			heading: Default::default(),
			ruder: Default::default(),
			velocity: Default::default(),
			resource_weight: 0,
			resource_value: 0,
			angle_of_list: 0.0,
		}
	}
}


/// Represents the type or upgrade level of the sail
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Enum)]
#[derive(Serialize, Deserialize)]
pub enum ShipHull {
	Small,
	Bigger,
}
// TODO: use the `#[default]` attribute one day instead
impl Default for ShipHull {
	fn default() -> Self {
		Self::Small
	}
}
impl ShipHull {
	pub fn upgrade(self) -> Option<Self> {
		use ShipHull::*;
		match self {
			Small => Some(Bigger),
			Bigger => None,
		}
	}

	pub fn value(self) -> u64 {
		use ShipHull::*;
		match self {
			Small => 1_000,
			Bigger => 2_000,
		}
	}
}

/// Represents the type or upgrade level of the sail
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Enum)]
#[derive(Serialize, Deserialize)]
pub enum SailKind {
	Cog,
	Bermuda,
	Schooner,
}
// TODO: use the `#[default]` attribute one day instead
impl Default for SailKind {
	fn default() -> Self {
		Self::Cog
	}
}
impl SailKind {
	/// Gives the next better sail kind, if any
	pub fn upgrade(self) -> Option<Self> {
		use SailKind::*;
		match self {
			Cog => Some(Bermuda),
			Bermuda => Some(Schooner),
			Schooner => None,
		}
	}

	/// Returns the nominal value of this sail (i.e. purchase cost)
	pub fn value(self) -> u64 {
		use SailKind::*;
		match self {
			Cog => 500,
			Bermuda => 1_000,
			Schooner => 2_000,
		}
	}

	/// Returns the highest supported reefing level of this sail.
	pub fn max_reefing(self) -> Reefing {
		let reefs = match self {
			Self::Cog => 3,
			Self::Bermuda => 4,
			Self::Schooner => 7,
		};
		Reefing(reefs)
	}

	/// Returns the sail area if at max reefing level.
	pub fn max_area(self) -> f32 {
		match self {
			// TODO: Maybe use 300, once lift-based sailing comes around
			Self::Cog => 100.,
			Self::Bermuda => 200.,
			Self::Schooner => 500.,
		}
	}
}

/// Represents the sail of the ship
#[derive(Debug, Default, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Sail {
	/// The sail type
	pub kind: SailKind,
	/// Current engagement of the break pedal (1.0 is full breaking, 0.0 is no-breaking)
	pub condition: Fraction,
	/// Current state of the gear box.
	pub reefing: Reefing,
	/// Absolute sail orientation for rectangle-rigged sails in radians, zero is word-X.
	pub orientation_rectangle: f32,
	/// Absolute sail orientation for triangle-rigged sails in radians, zero is word-X.
	pub orientation_triangle: f32,
}
impl Sail {
	/// Square rigged orientation as unit vector.
	pub fn orientation_rectangle_vec(&self) -> Vec2 {
		Vec2::new(
			self.orientation_rectangle.cos(),
			self.orientation_rectangle.sin(),
		)
	}

	/// Triangular rigged orientation as unit vector.
	pub fn orientation_triangle_vec(&self) -> Vec2 {
		Vec2::new(
			self.orientation_triangle.cos(),
			self.orientation_triangle.sin(),
		)
	}

	/// The currently deployed area of the sail.
	pub fn sail_area(self) -> f32 {
		let max_area = self.kind.max_area();
		let rel_sail = (f32::from(self.reefing.0) / f32::from(self.kind.max_reefing().0)).min(1.0);

		max_area * rel_sail.powi(2)
	}
}

/// Represents the dynamic state of a player
#[derive(Debug, Default, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Player {
	/// The vehicle of the player
	pub vehicle: Vehicle,
	/// The current money of the player
	pub money: u64,
}


/// Represents the currently deployed sail amount.
///
/// It influences the proportion of the wind that can be used to propel the ship.
///
/// The reefing levels range from zero (no sail) to some sail specific maximum,
/// see [SailKind].
///
/// Notice that gears are zero-indexed, thus `Gear::Forward(0)` is the first (and lowest) gear in forward direction.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Reefing(u8);

impl Reefing {
	/// Shift up a gear, may switch to forward
	pub fn increase(self) -> Self {
		Self(self.0.saturating_add(1))
	}

	/// Shift down a gear, may switch to reverse
	pub fn decrease(self) -> Self {
		Self(self.0.saturating_sub(1))
	}

	/// The plain reefing value.
	///
	/// A value of zero means no sail at all.
	/// A sail set is represented by the highs reefing value, which depends
	/// on the sail kind.
	pub fn value(self) -> u8 {
		self.0
	}
}
