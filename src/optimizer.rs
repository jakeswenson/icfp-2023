use std::collections::HashMap;
use mincost::{Particle, PsoConfig};
use multimap::MultiMap;
use rand::Rng;
use z3::ast::Ast;
use z3::Symbol;
use crate::models::{Attendee, Instrument, Position, ProblemSpec};


#[derive(Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct MusicianId(pub usize);

#[derive(Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct AttendeeId(usize);

pub fn optimize(problem: ProblemSpec) -> HashMap<MusicianId, Position> {

  let musicians: HashMap<MusicianId, Instrument> = problem.musicians.iter()
    .copied()
    .enumerate()
    .map(|(idx, inst)| (MusicianId(idx), inst))
    .collect();

  let attendees: HashMap<AttendeeId, Attendee> = problem.attendees.clone().into_iter()
    .enumerate()
    .map(|(idx, a)| (AttendeeId(idx), a))
    .collect();

  let config = z3::Config::new();
  let ctx = z3::Context::new(&config);
  let optimize = z3::Optimize::new(&ctx);

  type Score<'a> = z3::ast::Int<'a>;

  #[derive(PartialEq, Eq, Clone, Debug)]
  struct MusicianSymbols<'a> {
    x_var: Score<'a>,
    y_var: Score<'a>
  }

  let musician_symbol_map: HashMap<MusicianId, MusicianSymbols> = musicians.keys()
    .copied()
    .map(|m| {
      let x_symbol = Symbol::String(format!("M{}-X", m.0));
      let y_symbol = Symbol::String(format!("M{}-Y", m.0));
      let x_var = Score::new_const(&ctx, x_symbol);
      let y_var = Score::new_const(&ctx, y_symbol);
      (m, MusicianSymbols {
        x_var,
        y_var
      })
    }).collect();

  let attendee_tastes: HashMap<AttendeeId, HashMap<MusicianId, i64>> =  attendees.iter().map(|(a_id, a)| {
    (*a_id, musicians.iter().map(|(m, inst)| {
      (*m, 1_000_000i64.checked_mul(a.tastes[inst.0]).unwrap())
    }).collect())
  }).collect();

  let scores: Vec<Score<'_>> = attendees.iter().flat_map(|(attendees_id, a)| {
    let x = a.position.x as i64;
    let y = a.position.y as i64;
    let attendees_id = *attendees_id;

    musician_symbol_map.iter()
      .map(move |(musician_id, symbols)| {
        (attendees_id, x, y, *musician_id, symbols.clone())
      })
  }).map(|(attendees_id, x, y, musician_id, symbols)| {
    let x: i64 = x;
    let y: i64 = y;
    let x_dist = symbols.x_var.clone() - x;
    let y_dist = symbols.y_var.clone() - y;

    let dist_squared = x_dist.clone() * x_dist + y_dist.clone() * y_dist;
    let calculated_score = attendee_tastes
      .get(&attendees_id).unwrap()
      .get(&musician_id).copied().unwrap();

    let score = Score::from_i64(&ctx, calculated_score);

    let adjusted = score/dist_squared;

    adjusted
  }).collect();

  let x_start = (problem.stage_bottom_left[0] as i64) + 10;
  let y_start = (problem.stage_bottom_left[1] as i64) + 10;

  let x_end = x_start - 20 + (problem.room_width as i64);
  let y_end = y_start - 20 + (problem.stage_height as i64);

  let x_start = z3::ast::Int::from_i64(&ctx, x_start);
  let y_start = z3::ast::Int::from_i64(&ctx, y_start);
  let x_end = z3::ast::Int::from_i64(&ctx, x_end);
  let y_end = z3::ast::Int::from_i64(&ctx, y_end);

  musician_symbol_map.values().for_each(|syms| {
    optimize.assert(&syms.x_var.clone().ge(&x_start));
    optimize.assert(&syms.y_var.clone().ge(&y_start));

    optimize.assert(&syms.x_var.clone().le(&x_end));
    optimize.assert(&syms.y_var.clone().le(&y_end));
  });


  let total_score_symbol = Symbol::String("TotalScore".into());

  let total_score = Score::new_const(&ctx, total_score_symbol);

  let sum_of_scores = scores.into_iter().reduce(|a, b| a + b).unwrap().clone();

  dbg!(optimize.check(&[total_score._eq(&sum_of_scores)]));

  optimize.maximize(&sum_of_scores);

  let model = optimize.get_model().unwrap();

  musician_symbol_map.iter().map(|(&id, syms)| {
    let x = model.eval(&syms.x_var, true).and_then(|i| i.as_i64()).unwrap_or_default() as f32;
    let y = model.eval(&syms.y_var, true).and_then(|i| i.as_i64()).unwrap_or_default() as f32;

    (id, Position { x, y })
  }).collect()
}

pub fn particle_swarm_optimizer(problem: &ProblemSpec) -> HashMap<MusicianId, Position> {
  let mus_inst: HashMap<MusicianId, Instrument> = problem.musicians.iter().copied().enumerate()
    .map(|(idx, inst)| (MusicianId(idx), inst))
    .collect();

  let mut mus_map: HashMap<MusicianId, Position> = HashMap::new();

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

  for (inst, scores) in inst_scores {
    inst_score_functions.insert(inst, Box::new(move |pos| {
      if !(x_start..=x_end).contains(&pos.x)
        || !(y_start..=y_end).contains(&pos.y) {
        return 10.0;
      }
      scores.iter().map(|(a_pos, score)| {
        let top = (*score as f64) * 1_000_000.0f64;
        let del_x = a_pos.x - pos.x;
        let del_y = a_pos.y - pos.y;
        let dist_sq = del_x * del_x + del_y * del_y;

        -(top / (dist_sq as f64))
      }).sum()
    }));
  }

  // for (idx, inst) in problem.musicians.iter().enumerate() {
  //
  //   let x_step = 10.0f32;
  //   let y_step = 10.0f32;
  //
  //   let items_per_row = (problem.stage_width/10.0).floor() as usize - 1;
  //
  //   let x = x_start + (x_step * ((idx % items_per_row) as f32));
  //   let y = y_start + (y_step * ((idx / items_per_row) as f32));
  //
  //   mus_map.insert(MusicianId(idx), Position { x, y })
  // }

  for (mus, inst) in mus_inst {
    println!("Optimizing Musician: {:?}", mus);
    let mut opt = mincost::PsOpt::init(
      PsoConfig {
        pop_size: 100,
        omega: 0.1,
        phi_g: 0.1,
        phi_p: 0.1,
        learning_rate: 0.1,
        iteration: problem.attendees.len(),
      },
      |p| {
        let pos = Position { x: p[0], y: p[1] };
        let func = inst_score_functions.get(&inst).unwrap();

        func(pos)
      },
      || {
        let mut random = rand::thread_rng();
        let x = random.gen_range(x_start..=x_end);
        let y = random.gen_range(y_start..=y_end);

        Particle {
          position: vec![x, y],
          velocity: vec![0f32, 0f32],
          best_known_position: vec![x, y]
        }
      }
    );

    let position = opt.optimize();

    mus_map.insert(mus, Position { x: position[0], y: position[1] });
  }


  mus_map
}


