use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Sub;
use std::ops::SubAssign;

use nalgebra_glm::Vec2;
use serde::Deserialize;
use serde::Serialize;


/// An arbitrary distance on the map given in meters
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
pub struct Distance(pub Vec2);

impl Distance {
	pub fn new(x: f32, y: f32) -> Self {
		Self(Vec2::new(x, y))
	}

	pub fn magnitude(self) -> f32 {
		self.0.magnitude()
	}

	pub fn magnitude_sq(self) -> f32 {
		self.0.magnitude_squared()
	}
}
impl From<Vec2> for Distance {
	fn from(vec: Vec2) -> Self {
		Self(vec)
	}
}
impl From<Distance> for Vec2 {
	fn from(d: Distance) -> Self {
		d.0
	}
}
impl Add for Distance {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Distance(self.0 + rhs.0)
	}
}
impl AddAssign for Distance {
	fn add_assign(&mut self, rhs: Self) {
		self.0 += rhs.0;
	}
}
impl Sub for Distance {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Distance(self.0 - rhs.0)
	}
}
impl SubAssign for Distance {
	fn sub_assign(&mut self, rhs: Self) {
		self.0 -= rhs.0;
	}
}
impl Mul<f32> for Distance {
	type Output = Self;

	fn mul(self, rhs: f32) -> Self::Output {
		Self(self.0 * rhs)
	}
}
impl MulAssign<f32> for Distance {
	fn mul_assign(&mut self, rhs: f32) {
		self.0 *= rhs
	}
}
impl Div<f32> for Distance {
	type Output = Self;

	fn div(self, rhs: f32) -> Self::Output {
		Self(self.0 / rhs)
	}
}
impl DivAssign<f32> for Distance {
	fn div_assign(&mut self, rhs: f32) {
		self.0 /= rhs
	}
}

/// Represents wind conditions
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
pub struct Wind(pub Vec2);
impl Wind {
	pub fn angle(self) -> f32 {
		f32::atan2(self.0.y, self.0.x)
	}

	pub fn magnitude(self) -> f32 {
		self.0.magnitude()
	}

	pub fn from_polar(angle: f32, magnitude: f32) -> Self {
		Self(Vec2::new(angle.cos(), angle.sin()) * magnitude)
	}
}

/// An arbitrary location on the map given in meters
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
pub struct Location(pub Vec2);


impl Location {
	pub const ORIGIN: Self = Self::new(0.0, 0.0);

	pub const fn new(x: f32, y: f32) -> Self {
		Self(Vec2::new(x, y))
	}

	pub fn min(mut self, other: Location) -> Self {
		self.0 = nalgebra_glm::min2(&self.0, &other.0);
		self
	}

	pub fn max(mut self, other: Location) -> Self {
		self.0 = nalgebra_glm::max2(&self.0, &other.0);
		self
	}

	pub fn clamp(mut self, min: Location, max: Location) -> Self {
		self.0 = nalgebra_glm::clamp_vec(&self.0, &min.0, &max.0);
		self
	}
}
impl From<Vec2> for Location {
	fn from(vec: Vec2) -> Self {
		Self(vec)
	}
}
impl From<Location> for Vec2 {
	fn from(d: Location) -> Self {
		d.0
	}
}
impl Add<Distance> for Location {
	type Output = Self;

	fn add(self, rhs: Distance) -> Self::Output {
		Location(self.0 + rhs.0)
	}
}
impl AddAssign<Distance> for Location {
	fn add_assign(&mut self, rhs: Distance) {
		self.0 += rhs.0;
	}
}
impl Sub<Distance> for Location {
	type Output = Self;

	fn sub(self, rhs: Distance) -> Self::Output {
		Location(self.0 - rhs.0)
	}
}
impl SubAssign<Distance> for Location {
	fn sub_assign(&mut self, rhs: Distance) {
		self.0 -= rhs.0;
	}
}
impl Sub for Location {
	type Output = Distance;

	fn sub(self, rhs: Self) -> Self::Output {
		Distance(self.0 - rhs.0)
	}
}



/// A point it world time
///
/// A `Tick` has only meaning in the context of a specific game.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub struct Tick(pub u64);
impl Tick {
	pub fn next(self) -> Self {
		Self(self.0 + 1)
	}
}


/// Amount of fish in kilograms
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
pub struct Fish(pub f32);



/// A fractional value form in range `0.0..=1.0`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub struct Fraction(pub u8);
impl Fraction {
	pub fn from_f32(v: f32) -> Option<Self> {
		if (0.0..=1.0).contains(&v) {
			Some(Fraction((v * 255.0) as u8))
		} else {
			None
		}
	}

	pub fn to_f32(self) -> f32 {
		(self.0 as f32) / 255.0
	}
}
impl From<Fraction> for f32 {
	fn from(f: Fraction) -> Self {
		f.to_f32()
	}
}
impl Mul for Fraction {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		Fraction(((self.0 as u16 * rhs.0 as u16) / 255) as u8)
	}
}
impl MulAssign for Fraction {
	fn mul_assign(&mut self, rhs: Self) {
		*self = self.mul(rhs);
	}
}
impl Div for Fraction {
	type Output = Self;

	fn div(self, rhs: Self) -> Self::Output {
		Fraction((self.0 as u16 * 255 / rhs.0 as u16) as u8)
	}
}
impl DivAssign for Fraction {
	fn div_assign(&mut self, rhs: Self) {
		*self = self.div(rhs);
	}
}


/// A fractional value form in range `-1.0..=1.0`
// TODO: implement Default by hand, so that 0.0 is the default!
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub struct BiPolarFraction(pub i8);
impl BiPolarFraction {
	pub fn from_f32(v: f32) -> Option<Self> {
		if (-1.0..=1.0).contains(&v) {
			Some(BiPolarFraction((v * 127.0) as i8))
		} else {
			None
		}
	}

	pub fn to_f32(self) -> f32 {
		(self.0 as f32) / 127.0
	}
}
impl From<BiPolarFraction> for f32 {
	fn from(f: BiPolarFraction) -> Self {
		f.to_f32()
	}
}
impl Mul for BiPolarFraction {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		BiPolarFraction(((self.0 as i16 * rhs.0 as i16) / 127) as i8)
	}
}
impl MulAssign for BiPolarFraction {
	fn mul_assign(&mut self, rhs: Self) {
		*self = self.mul(rhs);
	}
}
impl Div for BiPolarFraction {
	type Output = Self;

	fn div(self, rhs: Self) -> Self::Output {
		BiPolarFraction((self.0 as i16 * 127 / rhs.0 as i16) as i8)
	}
}
impl DivAssign for BiPolarFraction {
	fn div_assign(&mut self, rhs: Self) {
		*self = self.div(rhs);
	}
}
