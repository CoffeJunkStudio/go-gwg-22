use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::ops::Rem;

use nalgebra_glm::Vec2;
use rand::Rng;
use rand::SeedableRng;
use serde::Deserialize;
use serde::Serialize;

use crate::terrain::TileCoord;
use crate::units::BiPolarFraction;
use crate::units::Fish;
use crate::units::Fraction;
use crate::units::Location;
use crate::units::Tick;
use crate::units::Wind;
use crate::Input;
use crate::ResourcePack;
use crate::ResourcePackContent;
use crate::StdRng;
use crate::WorldInit;
use crate::ENGINE_IDEAL_RPM;
use crate::ENGINE_POWER;
use crate::ENGINE_STALL_RPM;
use crate::FRICTION_CROSS_SPEED_FACTOR;
use crate::FRICTION_GROUND_SPEED_FACTOR;
use crate::FRICTION_MOTOR_FACTOR;
use crate::GEAR_BASE_RATION;
use crate::GEAR_RATIO_PROGRESSION;
use crate::MAX_TRACTION;
use crate::MAX_WIND_SPEED;
use crate::RESOURCE_PACK_FISH_SIZE;
use crate::TIRE_SPEED_PER_RPM;
use crate::VEHICLE_DEADWEIGHT;
use crate::VEHICLE_SIZE;
use crate::WIND_CHANGE_INTERVAL;



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
	// TODO add stuff
	Fishy,
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

pub const TICKS_PER_SECOND: u16 = 60;
const DELTA: f32 = 1_f32 / TICKS_PER_SECOND as f32;

impl WorldState {
	pub fn update(&mut self, init: &WorldInit, inputs: &Input) -> Vec<Event> {
		let mut events = Vec::new();

		// Increment timestamp
		self.timestamp = self.timestamp.next();

		// Apply user inputs
		self.player.vehicle.apply_input(inputs.clone());

		// Update wind
		self.wind = {
			let interval = u64::from(TICKS_PER_SECOND) * u64::from(WIND_CHANGE_INTERVAL);
			let earlier = self.timestamp.0 / interval;
			let later = earlier + 1;
			let offset = self.timestamp.0 - earlier * interval;

			let early = {
				let mut rng = StdRng::new(
					0xcafef00dd15ea5e5,
					0xa02bdbf7bb3c0a7ac28fa16a64abf96 ^ u128::from(init.seed) ^ u128::from(earlier),
				);

				let angle = rng.gen::<f32>() * std::f32::consts::TAU;
				let magnitude = rng.gen::<f32>() * MAX_WIND_SPEED;
				Wind::from_polar(angle, magnitude)
			};
			let late = {
				let mut rng = StdRng::new(
					0xcafef00dd15ea5e5,
					0xa02bdbf7bb3c0a7ac28fa16a64abf96 ^ u128::from(init.seed) ^ u128::from(later),
				);

				let angle = rng.gen::<f32>() * std::f32::consts::TAU;
				let magnitude = rng.gen::<f32>() * MAX_WIND_SPEED;
				Wind::from_polar(angle, magnitude)
			};

			let lerpy = nalgebra_glm::lerp(&early.0, &late.0, offset as f32 / interval as f32);
			Wind(lerpy)

			/*
			// Turning wind
			Wind::from_polar(
				(self.timestamp.0 % (u64::from(TICKS_PER_SECOND) * u64::from(WIND_CHANGE_INTERVAL)))
					as f32 / (u64::from(TICKS_PER_SECOND) * u64::from(WIND_CHANGE_INTERVAL)) as f32
					* std::f32::consts::TAU,
				1.0,
			)
			*/
		};

		//let water_consumption = crate::WATER_CONSUMPTION * DELTA;

		{
			let p = &mut self.player;

			// in s
			let duration = DELTA;

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

				let local_sail_angle =
					(normalize_angle_rel(local_wind_angle + PI)).clamp(-PI / 2., PI / 2.) - PI;
				p.vehicle.sail.orientation = local_sail_angle + ship_angle;


				let sail_drag_ness = 1.
					- p.vehicle
						.sail
						.orientation_vec()
						.dot(&apparent_wind.normalize())
						.abs();

				let sail_drag = apparent_wind * sail_drag_ness;


				let static_ship_area = 1.;
				let max_sail_area = 200.;
				let rel_area = match p.vehicle.sail.reefing {
					Reefing::Reefed(n) => (f32::from(n) / 4.).min(1.0),
				};
				let sail_area = max_sail_area * rel_area;

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

			// Move according to acceleration & velocity
			p.vehicle.velocity += acc * duration;
			let distance = duration * (vel_0 + duration * acc);

			let old_tile: TileCoord = p.vehicle.pos.try_into().expect("Player is out of bounds");
			p.vehicle.pos.0 += distance;


			// Terrain interaction
			if init.terrain.contains(p.vehicle.pos) {
				if Some(true) == init.terrain.try_get(old_tile).map(|t| t.is_passable()) {
					let new_tile: TileCoord =
						p.vehicle.pos.try_into().expect("Player goes out of bounds");

					match init.terrain.get(new_tile).is_passable() {
						true => {
							// Alright
						},
						false => {
							// TODO: maybe we want to handle this differently
							// Vehicles bounce off mountains
							p.vehicle.pos.0 -= distance;

							p.vehicle.velocity *= -0.5;

							if old_tile.x == new_tile.x {
								// restore x component sign
								p.vehicle.velocity.x *= -1.;
							}
							if old_tile.y == new_tile.y {
								// restore y component sign
								p.vehicle.velocity.y *= -1.;
							}
						},
					}
				}
			} else {
				// Player off map
				// Clamp
				p.vehicle.pos.0 -= distance;
				p.vehicle.velocity = Vec2::new(0., 0.);
			}


			/* TODO: how about a shore-based breaking
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

			p.vehicle.angle_of_list = (-(cross_speed / MAX_TRACTION / 2.) * PI).clamp(-PI,PI);

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
				let dist = VEHICLE_SIZE / 2.
					+ match r.content {
						ResourcePackContent::Fish => RESOURCE_PACK_FISH_SIZE / 2.,
					};

				if r.loc.0.metric_distance(&p.vehicle.pos.0) < dist {
					match r.content {
						ResourcePackContent::Fish => {
							p.vehicle.fish.0 += crate::RESOURCE_PACK_FISH_AMOUNT.0;
							events.push(Event::Fishy);
						},
					}

					false
				} else {
					true
				}
			});
		}

		events
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
	/// See [Input::steering]
	pub ruder: BiPolarFraction,
	/// State of the engine
	pub sail: Sail,
	//// Amount of fish on board
	pub fish: Fish,
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

	/// The raw engine RPM as if the engine never stalls.
	///
	/// Notice these RPMs can become negative.
	#[deprecated]
	fn engine_rpm_raw(&self) -> f32 {
		let axle_rpm = self.wheel_speed() / TIRE_SPEED_PER_RPM;

		let (gear, gear_dir): (u8, i8) = {
			match self.sail.reefing {
				Reefing::Reefed(n) => (n, 1),
			}
		};
		let gear_translation =
			GEAR_BASE_RATION * GEAR_RATIO_PROGRESSION.powi(gear.into()) * gear_dir as f32;

		axle_rpm / gear_translation
	}

	/// The current RPM of the engine
	///
	/// Returns `None` if the engine is stalling
	#[deprecated]
	pub fn engine_rpm(&self) -> Option<f32> {
		let rpm = self.engine_rpm_raw();

		// The first gear(s) never disengage
		if matches!(self.sail.reefing, Reefing::Reefed(0)) {
			return Some(rpm);
		}

		// Notice, if forward/reverse is wrongly selected,
		// the RPMs become even negative.
		if rpm > ENGINE_STALL_RPM {
			Some(rpm)
		} else {
			None
		}
	}

	/// The acceleration caused by friction in m/s
	///
	/// This acceleration is vectorial thus it can be just added to the `velocity`.
	pub fn friction_deacceleration(&self) -> Vec2 {
		let rolling_friction =
			-self.wheel_speed() * FRICTION_GROUND_SPEED_FACTOR * self.heading_vec();

		let sliding_friction =
			-self.cross_speed() * FRICTION_CROSS_SPEED_FACTOR * self.tangent_vec();

		let motor_friction = -self.engine_rpm().unwrap_or(0.0).abs()
			* FRICTION_MOTOR_FACTOR
			* self.wheel_speed().signum()
			* self.heading_vec();

		rolling_friction + sliding_friction + motor_friction
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
		VEHICLE_DEADWEIGHT + self.fish.0 as f32
	}
}

impl Default for Vehicle {
	fn default() -> Self {
		Self {
			pos: Default::default(),
			sail: Default::default(),
			heading: Default::default(),
			ruder: Default::default(),
			velocity: Default::default(),
			fish: Fish(0.0),
			angle_of_list: 0.0,
		}
	}
}

/// Represents the engine of a car
#[derive(Debug, Default, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Sail {
	/// Current engagement of the break pedal (1.0 is full breaking, 0.0 is no-breaking)
	pub condition: Fraction,
	/// Current state of the gear box.
	pub reefing: Reefing,
	/// Absolute sail orientation in radians, zero is word-X.
	pub orientation: f32,
}
impl Sail {
	/// Orientation as unit vector.
	pub fn orientation_vec(&self) -> Vec2 {
		Vec2::new(self.orientation.cos(), self.orientation.sin())
	}
}

/// Represents the dynamic state of a player
#[derive(Debug, Default, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Player {
	pub vehicle: Vehicle,
}


/// Represents the currently deployed sail amount.
///
/// It influences the proportion of the wind that can be
///
/// Notice that gears are zero-indexed, thus `Gear::Forward(0)` is the first (and lowest) gear in forward direction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub enum Reefing {
	/// Reefing level from zero (no sail) to some ship specific maximum.
	Reefed(u8),
}
impl Reefing {
	/// Shift up a gear, may switch to forward
	pub fn increase(self) -> Self {
		match self {
			Self::Reefed(n) => Self::Reefed(n.saturating_add(1)),
		}
	}

	/// Shift down a gear, may switch to reverse
	pub fn decrease(self) -> Self {
		match self {
			Self::Reefed(0) => Self::Reefed(0),
			Self::Reefed(n) => Self::Reefed(n - 1),
		}
	}
}
impl Default for Reefing {
	fn default() -> Self {
		// first gear forward
		Self::Reefed(0)
	}
}
