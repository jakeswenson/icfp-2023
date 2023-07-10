use std::collections::HashMap;
use ::z3::Symbol;
use z3::ast::Float;
use crate::models::{Attendee, Instrument, Position, ProblemSpec};
use super::{AttendeeId, MusicianId};

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

  let attendee_tastes: HashMap<AttendeeId, HashMap<MusicianId, f64>> =  attendees.iter().map(|(a_id, a)| {
    (*a_id, musicians.iter().map(|(m, inst)| {
      (*m, 1_000_000f64 * a.tastes[inst.0])
    }).collect())
  }).collect();

  let scores: Vec<Float<'_>> = attendees.iter().flat_map(|(attendees_id, a)| {
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

    let _dist_squared = x_dist.clone() * x_dist + y_dist.clone() * y_dist;
    let calculated_score = attendee_tastes
      .get(&attendees_id).unwrap()
      .get(&musician_id).copied().unwrap();

    let score = Float::from_f64(&ctx, calculated_score);

    //let adjusted = score/dist_squared;

   score
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


  let _total_score_symbol = Symbol::String("TotalScore".into());

 // let total_score = Float::new_const(&ctx, total_score_symbol);

  let sum_of_scores = scores.into_iter().reduce(|a, _b| a ).unwrap().clone();

 // dbg!(optimize.check(&[total_score._eq(&sum_of_scores)]));

  optimize.maximize(&sum_of_scores);

  let model = optimize.get_model().unwrap();

  musician_symbol_map.iter().map(|(&id, syms)| {
    let x = model.eval(&syms.x_var, true).and_then(|i| i.as_i64()).unwrap_or_default() as f32;
    let y = model.eval(&syms.y_var, true).and_then(|i| i.as_i64()).unwrap_or_default() as f32;

    (id, Position { x, y })
  }).collect()
}
