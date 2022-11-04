use nalgebra_glm::Vec2;
use serde::Deserialize;
use serde::Serialize;

use crate::units::BiPolarFraction;
use crate::units::Fish;
use crate::units::Fraction;
use crate::units::Location;
use crate::units::Tick;
use crate::Input;
use crate::ResourcePack;
use crate::ResourcePackContent;
use crate::TileCoord;
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
use crate::RESOURCE_PACK_FISH_SIZE;
use crate::TIRE_SPEED_PER_RPM;
use crate::VEHICLE_DEADWEIGHT;
use crate::VEHICLE_SIZE;



/// Events that can happen between ticks
#[derive(Debug, Clone)]
pub enum Event {
	// TODO add stuff
}



/// The dynamic part of the world
#[derive(Debug, Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct WorldState {
	/// The point in time of this state
	pub timestamp: Tick,
	/// The full list of active players
	pub player: Player,
	/// The full list of collectables on the map
	pub resources: Vec<ResourcePack>,
}

pub const TICKS_PER_SECOND: u16 = 20;
const DELTA: f32 = 1_f32 / TICKS_PER_SECOND as f32;

impl WorldState {
	pub fn update(&mut self, init: &WorldInit, inputs: &Input) -> Vec<Event> {
		// Incooperate inputs
		self.player.vehicle.apply_input(inputs.clone());


		// Remove dead players, i.e. those who don't have any water
		//self.players.retain(|_, p| p.vehicle.water.0 > 0.0);
		// TODO: what about a Game-Over condition

		//let water_consumption = crate::WATER_CONSUMPTION * DELTA;

		{
			let p = &mut self.player;

			// in s
			let duration = DELTA;

			// in m/s²
			let acceleration = if let Some(rpm) = p.vehicle.engine_rpm() {
				// Engine power

				// TODO: here is a feed-back loop during acceleration, when den RPMs rise and thus the power increases.
				let max_power = ENGINE_POWER;
				let rpm = rpm.clamp(ENGINE_STALL_RPM, ENGINE_IDEAL_RPM);
				let available_power = max_power * rpm / ENGINE_IDEAL_RPM;

				// as fraction
				// TODO: introduce wind (strength and direction)
				// TODO: use sail trim
				let throttle = 1.0;
				// in W
				let power = throttle * available_power;
				// in J
				let work = power * duration;

				// Acceleration

				// in m/s
				let speed = p.vehicle.ground_speed();
				// in kg
				let mass = p.vehicle.mass();

				// in m/s²
				let acceleration = (-speed + (speed * speed + 2.0 * work / mass).sqrt()) / duration;

				acceleration
			} else {
				// No user input
				0.0
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

			let acceleration = p.vehicle.heading_vec() * acceleration;
			let friction = p.vehicle.friction_deacceleration();

			let vel_0 = p.vehicle.velocity;

			let acc = acceleration + friction;

			// Move according to acceleration & velocity
			p.vehicle.velocity += acc * duration;
			let distance = duration * (vel_0 + duration * acc);

			let old_tile: TileCoord = p.vehicle.pos.into();
			p.vehicle.pos.0 += distance;


			// Terrain interaction
			if init.terrain.contains(p.vehicle.pos) {
				if init.terrain.get(old_tile).is_passable() {
					let new_tile: TileCoord = p.vehicle.pos.into();

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
				// TODO: what do we do here?
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

			p.vehicle.heading += angle * p.vehicle.ruder.to_f32().signum();

			// Turning by traction

			let head_speed = p.vehicle.wheel_speed();
			let cross_speed = p.vehicle.cross_speed();

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
						},
					}

					false
				} else {
					true
				}
			});
		}

		Vec::new()
	}
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
	/// Current heading as unit vector in world coordinates
	pub heading: f32,
	/// Gives the current steering.
	///
	/// Steering is always relative to `heading`.
	///
	/// See [Input::steering]
	pub ruder: BiPolarFraction,
	/// State of the engine
	pub engine: Sail,
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
	fn engine_rpm_raw(&self) -> f32 {
		let axle_rpm = self.wheel_speed() / TIRE_SPEED_PER_RPM;

		let (gear, gear_dir): (u8, i8) = {
			match self.engine.trim {
				Trim::Forward(n) => (n, 1),
				Trim::Reverse(n) => (n, -1),
			}
		};
		let gear_translation =
			GEAR_BASE_RATION * GEAR_RATIO_PROGRESSION.powi(gear.into()) * gear_dir as f32;

		axle_rpm / gear_translation
	}

	/// The current RPM of the engine
	///
	/// Returns `None` if the engine is stalling
	pub fn engine_rpm(&self) -> Option<f32> {
		let rpm = self.engine_rpm_raw();

		// The first gear(s) never disengage
		if matches!(self.engine.trim, Trim::Forward(0) | Trim::Reverse(0)) {
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
			trim: self.engine.trim,
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
			engine: Default::default(),
			heading: Default::default(),
			ruder: Default::default(),
			velocity: Default::default(),
			fish: Fish(10.0),
		}
	}
}

/// Represents the engine of a car
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Sail {
	/// Current engagement of the break pedal (1.0 is full breaking, 0.0 is no-breaking)
	pub condition: Fraction,
	/// Current state of the gear box.
	pub trim: Trim,
}

/// Represents the dynamic state of a player
#[derive(Debug, Default, Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Player {
	pub vehicle: Vehicle,
}


/// Represents the current gear.
///
/// Notice that gears are zero-indexed, thus `Gear::Forward(0)` is the first (and lowest) gear in forward direction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub enum Trim {
	/// Zero-indexed gear in forward direction (i.e. gives positive acceleration)
	Forward(u8),
	/// Zero-indexed gear in reverse direction (i.e. gives negative acceleration)
	Reverse(u8),
}
impl Trim {
	/// Shift up a gear, may switch to forward
	pub fn shift_up(self) -> Self {
		match self {
			Self::Forward(n) => Self::Forward(n + 1),
			Self::Reverse(0) => Self::Forward(0),
			Self::Reverse(n) => Self::Reverse(n - 1),
		}
	}

	/// Shift down a gear, may switch to reverse
	pub fn shift_down(self) -> Self {
		match self {
			Self::Forward(0) => Self::Reverse(0),
			Self::Forward(n) => Self::Forward(n - 1),
			Self::Reverse(n) => Self::Reverse(n + 1),
		}
	}
}
impl Default for Trim {
	fn default() -> Self {
		// first gear forward
		Self::Forward(0)
	}
}
