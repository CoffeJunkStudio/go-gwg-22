#[cfg(test)]
mod test;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default)]
pub struct Line(pub nalgebra::Point2<f32>, pub nalgebra::Point2<f32>);

impl Line {
	pub fn intersect(&self, other: &Self) -> Option<nalgebra::Point2<f32>> {
		let p1 = self.0;
		let p2 = self.1;
		let p3 = other.0;
		let p4 = other.1;

		let p1mp2 = p1 - p2;
		let p3mp4 = p3 - p4;

		let denom = p1mp2.x * p3mp4.y - p1mp2.y * p3mp4.x;
		let x1y2my1x2 = p1.x * p2.y - p1.y * p2.x;
		let x3y4my3x4 = p3.x * p4.y - p3.y * p4.x;

		let nom_x = x1y2my1x2 * p3mp4.x - x3y4my3x4 * p1mp2.x;
		let nom_y = x1y2my1x2 * p3mp4.y - x3y4my3x4 * p1mp2.y;

		let x = nom_x / denom;
		let y = nom_y / denom;

		(x.is_finite() && y.is_finite()).then(|| nalgebra::Point2::new(x, y))
	}
}
