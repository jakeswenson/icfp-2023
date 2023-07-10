use std::collections::HashMap;
use indicatif::{ProgressBar, ProgressStyle};
use mincost::{Particle, PsoConfig};
use multimap::MultiMap;
use rand::Rng;
use crate::models::{Attendee, Instrument, Position, ProblemSpec};
use parry2d::math::{Isometry, Point};
use parry2d::shape::{Ball, Segment};
use parry2d::bounding_volume::BoundingVolume;

pub mod z3;


#[derive(Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct MusicianId(pub usize);

#[derive(Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct AttendeeId(usize);

// Really 10 is supposed to be allowed, but I'm not sure if this works or not
const ALLOWED_MUSICIAN_DISTANCE: f32 = 10.5;

pub fn particle_swarm_optimizer(problem: &ProblemSpec) -> HashMap<MusicianId, Position> {
  let mus_inst: HashMap<MusicianId, Instrument> = problem.musicians.iter().copied().enumerate()
    .map(|(idx, inst)| (MusicianId(idx), inst))
    .collect();

  let x_start = problem.stage_bottom_left[0] + 10.0;
  let y_start = problem.stage_bottom_left[1] + 10.0;

  let x_end = problem.stage_bottom_left[0] + problem.stage_width - 10.0;
  let y_end = problem.stage_bottom_left[1] + problem.stage_height - 10.0;

  let attendees: HashMap<usize, Attendee> = problem.attendees.iter().cloned().enumerate().collect();

  let mut instrument_tastes: MultiMap<Instrument, (Position, f64)> = multimap::MultiMap::new();

  for (_id, attendee) in attendees {
    for (inst, &score) in attendee.tastes.iter().enumerate() {
      instrument_tastes.insert(Instrument(inst), (attendee.position, score));
    }
  }

  fn dist(p1: &Position, p2: &Position) -> f32 {
    let del_x = p1.x - p2.x;
    let del_y = p1.y - p2.y;

    f32::sqrt(del_x * del_x + del_y * del_y)
  }

  let mut inst_score_functions: HashMap<Instrument, Box<dyn Fn(HashMap<MusicianId, Position>) -> f64>> = HashMap::new();
  inst_score_functions.insert(Instrument(0), Box::new(move |all_musicians| {

    let mut score: f64 = 0.0;

    for (musician_id, musician_pos) in all_musicians.iter() {
      if !(x_start..=x_end).contains(&musician_pos.x)
        || !(y_start..=y_end).contains(&musician_pos.y) {
        return f64::MAX
      }

      for other in all_musicians.values() {
        if dist(&musician_pos, other) <= ALLOWED_MUSICIAN_DISTANCE {
          return f64::MAX
        }
      }

      let inst = mus_inst.get(&musician_id).unwrap();
      let audience_tastes = instrument_tastes.get_vec(inst).unwrap();

      score += audience_tastes.iter().map(|(a_pos, taste)| {

        // check if any musician other is in the way between pos and a_pos
        let line_start = Point::new(a_pos.x, a_pos.y);
        let line_end = Point::new(musician_pos.x, musician_pos.y);
        let segment = Segment::new(line_start, line_end);
        let segment_aabb = segment.aabb(&Isometry::translation(line_start.x, line_start.y));
        let circle_radius = 5.0;
        let circle_shape = Ball::new(circle_radius);

        for other in all_musicians.values() {
          if musician_pos == other {
            continue
          } else {
            let circle_center = Point::new(other.x, other.y);
            let circle_aabb = circle_shape.aabb(&Isometry::translation(circle_center.x, circle_center.y));

            if segment_aabb.intersects(&circle_aabb) {
              return 0.0
            }
          }
        }

        let top = (*taste as f64) * 1_000_000.0f64;
        let del_x = a_pos.x - musician_pos.x;
        let del_y = a_pos.y - musician_pos.y;
        let dist_sq = del_x * del_x + del_y * del_y;

        let score: f64 = -(top / (dist_sq as f64));
        score
      }).sum::<f64>()
    }

    score
  }));

  let particle_count: usize = 200;
  let num_iterations: usize = 10_000;

  let pb = ProgressBar::new((particle_count * num_iterations * 2) as u64);

  let sty = ProgressStyle::with_template(
    "{spinner:.green} [{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>3}/{len} {msg}",
  )
    .unwrap()
    .progress_chars("#>-");

  pb.set_style(sty);


  let mut opt = mincost::PsOpt::init(
    PsoConfig {
      pop_size: particle_count,
      omega: 1.0,
      phi_g: 0.2,
      phi_p: 0.1,
      learning_rate: 0.6,
      iteration: num_iterations,
    },
    |p| {
      pb.inc(1);

      let musician_map: HashMap<MusicianId, Position> = p.chunks(2)
        .enumerate()
        .map(|(id, p)| {
          (MusicianId(id), Position { x: p[0], y: p[1] })
        })
        .collect();

      let func = inst_score_functions.get(&Instrument(0)).unwrap();

      func(musician_map)
    },
    || {
      let mut random = rand::thread_rng();

      let musician_count = problem.musicians.len();
      let mut all_musicians = Vec::with_capacity(musician_count * 2);

      let mut claimed_positions = Vec::with_capacity(musician_count);

      for _i in 0..musician_count {
        let mut x ;
        let mut y ;
        let mut position;

        'find_pos: loop {
          x = random.gen_range(x_start..=x_end);
          y = random.gen_range(y_start..=y_end);
          position = Position { x, y };

          for already_claimed in claimed_positions.iter() {
            if dist(&position, already_claimed) <= ALLOWED_MUSICIAN_DISTANCE {
              continue 'find_pos
            }
          }

          break;
        }

        all_musicians.push(x);
        all_musicians.push(y);
        claimed_positions.push(position)
      }

      Particle {
        position: all_musicians.clone(),
        velocity: vec![0f32; musician_count * 2],
        best_known_position: all_musicians
      }
    }
  );

  let best_position_for_all_musicians = opt.optimize();

  pb.finish_with_message("Optimized");

  let musician_map: HashMap<MusicianId, Position> = best_position_for_all_musicians.chunks(2)
    .enumerate()
    .map(|(id, p)| {
      (MusicianId(id), Position { x: p[0], y: p[1] })
    })
    .collect();

  musician_map
}


