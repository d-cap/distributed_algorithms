use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(mouse_position_system)
        .add_system(mouse_click_system)
        .run();
}

#[derive(Component, Debug)]
struct CursorPosition(Vec2);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn().insert(CursorPosition(Vec2::ZERO));
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("red_dragon.png"),
        ..default()
    });
}

fn mouse_click_system(
    mut commands: Commands,
    mouse: Res<Input<MouseButton>>,
    asset_server: Res<AssetServer>,
    query: Query<&CursorPosition>,
) {
    if mouse.just_released(MouseButton::Left) {
        for c in query.iter() {
            commands.spawn_bundle(SpriteBundle {
                texture: asset_server.load("red_dragon.png"),
                transform: Transform::from_xyz(c.0.x, c.0.y, 0.),
                ..default()
            });
        }
    }
}

fn mouse_position_system(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut query: Query<&mut CursorPosition>,
) {
    for m in cursor_moved_events.iter() {
        for mut c in query.iter_mut() {
            *c.0 = *m.position;
        }
    }
}
