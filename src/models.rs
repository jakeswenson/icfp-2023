use serde::{Serialize, Deserialize};

pub type Dimension = f32;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize)]
pub struct Instrument(pub usize);

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Attendee {
  #[serde(flatten)]
  pub position: Position,
  pub tastes: Vec<i64>
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ProblemSpec {
  pub room_height: Dimension,
  pub room_width: Dimension,
  pub stage_height: Dimension,
  pub stage_width: Dimension,
  pub stage_bottom_left: [Dimension; 2],
  pub musicians: Vec<Instrument>,
  pub attendees: Vec<Attendee>
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Position {
  pub x: Dimension,
  pub y: Dimension,
}

#[derive(Serialize)]
pub struct Solution {
  pub placements: Vec<Position>
}
