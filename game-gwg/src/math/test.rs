use nalgebra as ng;

use super::Line;

const fn p(x: f32, y: f32) -> ng::Point2<f32> {
	ng::Point2::new(x, y)
}

#[test]
fn intersect_lines_at_origin() {
	// Arrange
	let a = Line(p(-1.0, 0.0), p(1.0, 0.0));
	let b = Line(p(0.0, -1.0), p(0.0, 1.0));

	// Act
	let actual = a.intersect(&b);

	// Assert
	let expected = p(0.0, 0.0);
	assert!(actual.is_some());
	assert!(logic::glm::distance(&actual.unwrap().coords, &expected.coords) < f32::EPSILON);
}

#[test]
fn intersect_lines_top() {
	// Arrange
	let a = Line(p(0.0, 600.0), p(800.0, 600.0));
	let b = Line(p(400.0, 300.0), p(400.0, 600.0));

	// Act
	let actual = a.intersect(&b);

	// Assert
	let expected = p(400.0, 600.0);
	assert!(actual.is_some());
	assert!(logic::glm::distance(&actual.unwrap().coords, &expected.coords) < f32::EPSILON);
}

#[test]
fn intersect_lines_bottom() {
	// Arrange
	let a = Line(p(0.0, 0.0), p(800.0, 0.0));
	let b = Line(p(400.0, 300.0), p(400.0, 0.0));

	// Act
	let actual = a.intersect(&b);

	// Assert
	let expected = p(400.0, 0.0);
	assert!(actual.is_some());
	assert!(logic::glm::distance(&actual.unwrap().coords, &expected.coords) < f32::EPSILON);
}

#[test]
fn intersect_lines_left() {
	// Arrange
	let a = Line(p(0.0, 0.0), p(0.0, 600.0));
	let b = Line(p(400.0, 300.0), p(600.0, 300.0));

	// Act
	let actual = a.intersect(&b);

	// Assert
	let expected = p(000.0, 300.0);
	assert!(actual.is_some());
	assert!(logic::glm::distance(&actual.unwrap().coords, &expected.coords) < f32::EPSILON);
}

#[test]
fn intersect_lines_right() {
	// Arrange
	let a = Line(p(800.0, 0.0), p(800.0, 600.0));
	let b = Line(p(400.0, 300.0), p(600.0, 300.0));

	// Act
	let actual = a.intersect(&b);

	// Assert
	let expected = p(800.0, 300.0);
	assert!(actual.is_some());
	assert!(logic::glm::distance(&actual.unwrap().coords, &expected.coords) < f32::EPSILON);
}
