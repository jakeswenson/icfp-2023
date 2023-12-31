use std::path::{PathBuf};
use crate::models::{Position, ProblemSpec, Solution};
use clap::{Parser, Subcommand};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::blocking::Response;
use serde::Serialize;
use crate::optimizer::MusicianId;

mod models;
mod render;
mod optimizer;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Runs optimizer on provided problem
  Optimize {
    problem: PathBuf,
  },
  /// Runs Renderer on provided problem
  Render {
    problem: PathBuf,
    #[arg(short, long)]
    solution: Option<PathBuf>
  },
  /// Runs Particle Swarm Optimizer on the provided problem
  Swarm {
    problem: PathBuf,
    #[arg(short, long)]
    render: bool,
    #[arg(short, long)]
    submit: Option<u32>,
  },
}

fn main() -> Result<(), anyhow::Error> {
  dotenvy::dotenv()?;
  let cli: Cli = Cli::parse();

  match &cli.command {
    Commands::Optimize { problem } => {
      let json = std::fs::read_to_string(problem)?;
      let problem_spec: ProblemSpec = serde_json::from_str(&json)?;
      dbg!(optimizer::z3::optimize(problem_spec));
    }
    Commands::Render { problem, solution } => {
      let json = std::fs::read_to_string(problem)?;
      let problem_spec: ProblemSpec = serde_json::from_str(&json)?;
      render::run_app(problem_spec, solution.clone().map(|sol| {
        let json = std::fs::read_to_string(sol).unwrap();
        let solution: Solution = serde_json::from_str(&json).unwrap();
        solution.placements.into_iter().enumerate()
          .map(|(id, p)| (MusicianId(id), p)).collect()
      } ));
    }
    Commands::Swarm { problem, render, submit } => {
      let json = std::fs::read_to_string(problem)?;
      let problem_spec: ProblemSpec = serde_json::from_str(&json)?;
      let result = optimizer::particle_swarm_optimizer(&problem_spec);

      let mut ordered: Vec<Position> = Vec::with_capacity(result.len());

      for idx in 0..result.len() {
        let v = result.get(&MusicianId(idx)).unwrap();
        ordered.push(Position { x: v.x, y: v.y });
      }

      let solution = Solution {
        placements: ordered
      };

      std::fs::write(format!("solution-{}", problem.file_name().unwrap().to_str().unwrap()), &serde_json::to_vec(&solution)?)?;


      if let Some(problem_id) = submit {
        let response = Submitter::submit(*problem_id, &solution)?;
        println!("Response code: {}", response.status())
      }

      if *render {
        render::run_app(problem_spec, Some(result))
      }
    }
  }

  Ok(())
}

struct Submitter;

impl Submitter {
  fn submit(problem_id: u32, solution: &Solution) -> Result<Response, anyhow::Error> {
    let mut header_map = HeaderMap::new();

    let token = std::env::var("API_TOKEN")?;

    header_map.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", token))?);

    let client = reqwest::blocking::Client::builder()
      .default_headers(header_map)
      .build()?;

    #[derive(Serialize)]
    struct Payload {
      problem_id: u32,
      contents: String
    }

    let response = client.post("https://api.icfpcontest.com/submission")
      .json(&Payload {
        problem_id,
        contents: serde_json::to_string(&solution)?
      })
      .send()?;

    Ok(response)
  }
}

#[cfg(test)]
mod tests {
  use crate::ProblemSpec;
  const PROBLEM_JSON: &str = include_str!("problems/example.json");

  #[test]
  fn parse_problem() {
    let problem:ProblemSpec = serde_json::from_str(PROBLEM_JSON).unwrap();

    assert_eq!(problem.room_width, 2000.0);
    assert_eq!(problem.room_height, 5000.0);
    assert_eq!(problem.stage_width, 1000.0);
    assert_eq!(problem.attendees.len(), 3);
    println!("{:?}", problem);
  }

  #[test]
  fn render_problem() {

  }

}
