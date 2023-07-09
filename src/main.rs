use std::path::{PathBuf};
use crate::models::ProblemSpec;
use clap::{Parser, Subcommand};

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
  },
  /// Runs Renderer on provided problem
  Pso {
    problem: PathBuf,
    #[arg(short, long)]
    render: bool
  },
}

fn main() -> Result<(), anyhow::Error> {
  let cli: Cli = Cli::parse();

  match &cli.command {
    Commands::Optimize { problem } => {
      let json = std::fs::read_to_string(problem)?;
      let problem_spec: ProblemSpec = serde_json::from_str(&json)?;
      dbg!(optimizer::optimize(problem_spec));
    }
    Commands::Render { problem } => {
      let json = std::fs::read_to_string(problem)?;
      let problem_spec: ProblemSpec = serde_json::from_str(&json)?;
      render::run_app(dbg!(problem_spec), None);
    }
    Commands::Pso { problem, render } => {
      let json = std::fs::read_to_string(problem)?;
      let problem_spec: ProblemSpec = serde_json::from_str(&json)?;
      let result = optimizer::particle_swarm_optimizer(&problem_spec);

      if *render {
        render::run_app(problem_spec, Some(result))
      }
    }
  }

  Ok(())
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
