use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use indicatif::{ProgressBar, ProgressStyle};
use mincost::{Particle, PsoConfig};
use multimap::MultiMap;
use rand::Rng;
use crate::models::{Attendee, Instrument, Position, ProblemSpec};

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

  let mus_map: HashMap<MusicianId, Position> = HashMap::new();

  let x_start = problem.stage_bottom_left[0] + 10.0;
  let y_start = problem.stage_bottom_left[1] + 10.0;

  let x_end = problem.stage_bottom_left[0] + problem.stage_width - 10.0;
  let y_end = problem.stage_bottom_left[1] + problem.stage_height - 10.0;

  let attendees: HashMap<usize, Attendee> = problem.attendees.iter().cloned().enumerate().collect();


  let mut inst_scores: MultiMap<Instrument, (Position, i64)> = multimap::MultiMap::new();

  for (_id, attendee) in attendees {
    for (inst, &score) in attendee.tastes.iter().enumerate() {
      inst_scores.insert(Instrument(inst), (attendee.position, score));
    }
  }

  let mut inst_score_functions: HashMap<Instrument, Box<dyn Fn(Position) -> f64>> = HashMap::new();
  let musician_position_state_map = Rc::new(RefCell::new(mus_map));

  fn dist(p1: &Position, p2: &Position) -> f32 {
    let del_x = p1.x - p2.x;
    let del_y = p1.y - p2.y;

    f32::sqrt(del_x * del_x + del_y * del_y)
  }

  for (inst, scores) in inst_scores {
    let m = Rc::clone(&musician_position_state_map);
    inst_score_functions.insert(inst, Box::new(move |pos| {
      if !(x_start..=x_end).contains(&pos.x)
        || !(y_start..=y_end).contains(&pos.y) {
        return f64::MAX
      }

      for other in m.borrow().values() {
        if dist(&pos, other) <= ALLOWED_MUSICIAN_DISTANCE {
          return f64::MAX
        }
      }

      scores.iter().map(|(a_pos, taste)| {
        let top = (*taste as f64) * 1_000_000.0f64;
        let del_x = a_pos.x - pos.x;
        let del_y = a_pos.y - pos.y;
        let dist_sq = del_x * del_x + del_y * del_y;

        -(top / (dist_sq as f64))
      }).sum()
    }));
  }

  let pb = ProgressBar::new(problem.musicians.len() as u64 + 1);

  let sty = ProgressStyle::with_template(
    "{spinner:.green} [{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>3}/{len} {msg}",
  )
    .unwrap()
    .progress_chars("#>-");

  pb.set_style(sty);

  for (mus, inst) in mus_inst {
    pb.set_message(format!("Optimizing {:?}", mus));
    pb.inc(1);

    let mut opt = mincost::PsOpt::init(
      PsoConfig {
        pop_size: 10,
        omega: 1.0,
        phi_g: 0.1,
        phi_p: 0.1,
        learning_rate: 0.6,
        iteration: 1000,
      },
      |p| {
        let pos = Position { x: p[0], y: p[1] };
        let func = inst_score_functions.get(&inst).unwrap();

        func(pos)
      },
      || {
        let mut random = rand::thread_rng();
        let x ;
        let y ;

        loop {
          x = random.gen_range(x_start..=x_end);
          y = random.gen_range(y_start..=y_end);

          for other in musician_position_state_map.borrow().values() {
            if dist(&Position{x,y}, other) <= ALLOWED_MUSICIAN_DISTANCE {
              continue
            }
          }

          break;
        }

        Particle {
          position: vec![x, y],
          velocity: vec![0f32, 0f32],
          best_known_position: vec![x, y]
        }
      }
    );

    let best_position_for_musician = opt.optimize();

    musician_position_state_map.borrow_mut().insert(mus, Position { x: best_position_for_musician[0], y: best_position_for_musician[1] });
  }

  pb.finish_with_message("Optimized");

  let result = musician_position_state_map.borrow();
  result.clone()
}


