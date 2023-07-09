use std::collections::HashMap;
use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy::app::CoreSet::Update;
use bevy::input::common_conditions::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use crate::models::{Position, ProblemSpec};
use crate::optimizer::MusicianId;

#[derive(Component)]
struct Camera;

#[derive(Component)]
struct Room;

#[derive(Component)]
struct Stage;

#[derive(Resource)]
struct Problem(ProblemSpec);

#[derive(Resource)]
struct ASolution(Option<HashMap<MusicianId, Position>>);

fn setup(
  problem: Res<Problem>,
  solution: Res<ASolution>,
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

  let max_inst = problem.musicians.iter().max().unwrap().0 + 1;

  // Room
  commands.spawn((MaterialMesh2dBundle {
    mesh: meshes
      .add(shape::Quad::new(Vec2::new(problem.room_width, problem.room_height)).into())
      .into(),
    material: materials.add(ColorMaterial::from(Color::BLACK)),
    transform: Transform::from_xyz(problem.room_width / 2.0, problem.room_height/2.0, 0.0),
    ..default()
  }, Room)).with_children(|parent| {
    parent.spawn(
      Text2dBundle {
        text: Text::from_section(format!("Height: {}", problem.room_height), text_style.clone())
          .with_alignment(TextAlignment::Left),
        transform: Transform::from_xyz(problem.room_width/2.0 + 100.0, 0.0, 0.0),
        ..default()
      });

    parent.spawn(
      Text2dBundle {
        text: Text::from_section(format!("Width: {}", problem.room_width), text_style.clone())
          .with_alignment(TextAlignment::Center),
        transform: Transform::from_xyz(0.0, -problem.room_height/2.0 - 100.0, 0.0),
        ..default()
      });
  });

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
        text: Text::from_section(format!("Stage {}x{}", problem.stage_width, problem.stage_height), text_style.clone())
          .with_alignment(TextAlignment::Center),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.2)),
        ..default()
      });
  });

  let red_color = materials.add(ColorMaterial::from(Color::RED));

  for attendee in problem.attendees.iter() {
    // println!("Adding attendee: {:?}", attendee);
    commands.spawn(MaterialMesh2dBundle {
      mesh: meshes.add(shape::RegularPolygon::new(1., 6).into()).into(),
      material: red_color.clone(),
      transform: Transform::from_translation(Vec3::new(attendee.position.x, attendee.position.y, 0.3)),
      ..default()
    });
  }

  commands.spawn(MaterialMesh2dBundle {
    mesh: meshes.add(shape::RegularPolygon::new(1., 3).into()).into(),
    material: materials.add(ColorMaterial::from(Color::YELLOW_GREEN)),
    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    ..default()
  });

  match &solution.0 {
    Some(solution) => {
      for (idx, pos) in solution {
        let instrument = problem.musicians[idx.0];
        let color = colorous::RAINBOW.eval_rational(instrument.0, max_inst);
        let x = pos.x;
        let y = pos.y;

        commands.spawn(MaterialMesh2dBundle {
          mesh: meshes.add(shape::Circle::new(5.0).into()).into(),
          material: materials.add(ColorMaterial::from(Color::rgb_u8(color.r, color.g, color.b))),
          transform: Transform::from_translation(Vec3::new(x, y, 10.0)),
          ..default()
        });
      }
    },
    _ => {
      for (idx, inst) in problem.musicians.iter().enumerate() {
        let color = colorous::RAINBOW.eval_rational(inst.0, max_inst);

        let x_start = problem.stage_bottom_left[0] + 10.0;
        let y_start = problem.stage_bottom_left[1] + 10.0;

        let x_step = 10.0f32;
        let y_step = 10.0f32;

        let items_per_row = (problem.stage_width/10.0).floor() as usize - 1;

        let x = x_step * ((idx % items_per_row) as f32);
        let y = y_step * ((idx / items_per_row) as f32);

        commands.spawn(MaterialMesh2dBundle {
          mesh: meshes.add(shape::Circle::new(5.0).into()).into(),
          material: materials.add(ColorMaterial::from(Color::rgb_u8(color.r, color.g, color.b))),
          transform: Transform::from_translation(Vec3::new(x_start + x, y_start + y, 10.0)),
          ..default()
        });
      }
    }
  }
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
        // println!("Scroll (line units): vertical: {}, horizontal: {}", ev.y, ev.x);
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
  projection_query: Query<&mut OrthographicProjection, With<Camera>>,
  mut q: Query<&mut Transform, With<Camera>>,
  mut motion_evr: EventReader<MouseMotion>,
) {
  let projection = projection_query.single();

  let mut transform = q.single_mut();
  for ev in motion_evr.iter() {
    let scale = projection.scale.clamp(1.0, 5.0);
    transform.translation.x -= ev.delta.x * scale;
    transform.translation.y += ev.delta.y * scale;
  }
}

pub(crate) fn run_app(problem_spec: ProblemSpec, solution: Option<HashMap<MusicianId, Position>>) {
  App::new()
    // Background Color
    // https://bevy-cheatbook.github.io/window/clear-color.html
    .insert_resource(ClearColor(Color::GRAY))
    .insert_resource(Problem(problem_spec))
    .insert_resource(ASolution(solution))
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
