use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

const N: usize = 8;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(ClearColor(Color::rgb(0.6, 0.6, 0.6)))
        .add_startup_system(setup)
        .add_system(color_variation)
        .run();
}

#[derive(Component)]
struct Cube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // set up the camera
    let mut camera = OrthographicCameraBundle::new_3d();
    camera.orthographic_projection.scale = 3.0;
    camera.transform = Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y);

    // camera
    commands.spawn_bundle(camera);

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    let radius = 2.;
    // cubes
    for i in 0..N {
        let v = i as f32 * (std::f32::consts::PI * 2.) / N as f32;
        let size = 0.6;
        let bundle = PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(radius * v.cos(), size / 2., radius * v.sin()),
            ..default()
        };
        commands.spawn_bundle(bundle);
    }
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(3.0, 8.0, 5.0),
        point_light: PointLight {
            intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
            color: Color::WHITE,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

fn color_variation(time: Res<Time>, mut query: Query<&mut Handle<StandardMaterial>, With<Cube>>) {
    for mut material in query.iter_mut() {
        println!("{:#?}", material);
        // material. *= Quat::from_rotation_x(3.0 * time.delta_seconds());
    }
}
