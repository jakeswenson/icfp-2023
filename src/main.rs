mod models;
mod render;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle, app::CoreSet};
use bevy::app::CoreSet::Update;
use bevy::input::common_conditions::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::transform::TransformSystem;
use models::{ProblemSpec};

#[derive(Component)]
struct Camera;

#[derive(Component)]
struct Room;

#[derive(Component)]
struct Stage;

#[derive(Resource)]
struct Problem(ProblemSpec);

fn setup(
  problem: Res<Problem>,
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
  asset_server: Res<AssetServer>
) {
  let problem: &ProblemSpec = &problem.0;

  let font = asset_server.load("fonts/FiraSans-Bold.ttf");
  let text_style = TextStyle {
    font: font.clone(),
    font_size: 60.0,
    color: Color::WHITE,
  };

  commands.spawn((Camera2dBundle::new_with_far(100.), Camera));

  // Room
  commands.spawn((MaterialMesh2dBundle {
    mesh: meshes
      .add(shape::Quad::new(Vec2::new(problem.room_width, problem.room_height)).into())
      .into(),
    material: materials.add(ColorMaterial::from(Color::TEAL)),
    transform: Transform::from_xyz(problem.room_width / 2.0, problem.room_height/2.0, 0.0),
    ..default()
  }, Room));

  // Stage
  commands.spawn((MaterialMesh2dBundle {
    mesh: meshes
      .add(shape::Quad::flipped(Vec2::new(problem.stage_width, problem.stage_height)).into())
      .into(),
    material: materials.add(ColorMaterial::from(Color::LIME_GREEN)),
    transform: Transform::from_xyz(
      problem.stage_bottom_left[0] + (problem.stage_width/2.0),
      problem.stage_bottom_left[1] + (problem.stage_height/2.0),
      0.1),
    ..default()
  }, Stage)).with_children(|parent| {
    parent.spawn(
      Text2dBundle {
        text: Text::from_section("Stage", text_style.clone())
          .with_alignment(TextAlignment::Center),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.2)),
        ..default()
      });
  });

  let handle = materials.add(ColorMaterial::from(Color::PURPLE));

  for attendee in problem.attendees.iter() {
    println!("Adding attendee: {:?}", attendee);
    commands.spawn(MaterialMesh2dBundle {
      mesh: meshes.add(shape::RegularPolygon::new(15., 6).into()).into(),
      material: handle.clone(),
      transform: Transform::from_translation(Vec3::new(attendee.position.x, attendee.position.y, 0.3)),
      ..default()
    });
  }

  commands.spawn(MaterialMesh2dBundle {
    mesh: meshes.add(shape::RegularPolygon::new(5., 3).into()).into(),
    material: materials.add(ColorMaterial::from(Color::RED)),
    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    ..default()
  });
}

fn debug_globaltransform(
  query: Query<&GlobalTransform, With<Room>>,
) {
  let gxf = query.single();
  debug!("Room at: {:?}", gxf.translation());
}


//https://bevy-cheatbook.github.io/input/mouse.html
// https://bevy-cheatbook.github.io/features/camera.html
fn zoom_camera(
  mut q: Query<&mut OrthographicProjection, With<Camera>>,
  mut scroll_evr: EventReader<MouseWheel>,
) {
  use bevy::input::mouse::MouseScrollUnit;
  for ev in scroll_evr.iter() {
    match ev.unit {
      MouseScrollUnit::Line => {
        println!("Scroll (line units): vertical: {}, horizontal: {}", ev.y, ev.x);
      }
      MouseScrollUnit::Pixel => {
        let mut projection = q.single_mut();

        projection.scale *= 1.0 - (ev.y/1000.);

        // always ensure you end up with sane values
        // (pick an upper and lower bound for your application)
        projection.scale = projection.scale.clamp(0.2, 100.0);
      }
    }
  }
}

fn move_camera(
  mut q: Query<&mut Transform, With<Camera>>,
  mut motion_evr: EventReader<MouseMotion>,
) {

  let mut projection = q.single_mut();
  for ev in motion_evr.iter() {
    projection.translation.x -= ev.delta.x * 2.0;
    projection.translation.y += ev.delta.y * 2.0;
  }
}

fn main() {
  let problem_json = include_str!("../problems/example.json");
  let problem_spec = serde_json::from_str(problem_json).unwrap();

  App::new()
    // Background Color
    // https://bevy-cheatbook.github.io/window/clear-color.html
    .insert_resource(ClearColor(Color::GRAY))
    .insert_resource(Problem(problem_spec))
    .add_plugins(DefaultPlugins)
    .add_startup_system(setup)
    .add_system(zoom_camera)
    .add_system(
      move_camera
        .in_base_set(Update)
        .run_if(input_pressed(MouseButton::Left))
    )
    // .add_system(
    //   debug_globaltransform
    //     .in_base_set(CoreSet::PostUpdate)
    //     .after(TransformSystem::TransformPropagate)
    // )
    .run();
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
