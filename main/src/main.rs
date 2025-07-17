use bevy::prelude::*;

fn main() {
    App::new()
        .add_systems(Startup, add_entities)
        .add_systems(Update, (hello_world, (change_position, print_position).chain()))
        .run();
    println!("Hello, world!");
}

fn hello_world() {
    println!("SYSTEM MESSAGE: Hello, world!");
}

// Components
#[derive(Component)]
struct Position {
    x: f64,
    y: f64,
}

#[derive(Component)]
struct Entity;


// Systems
fn add_entities(mut commands: Commands) {
    commands.spawn((Entity, Position { x: 1.0, y: 2.0 } ));
    commands.spawn((Entity, Position { x: 2.0, y: 3.0 } ));
    commands.spawn((Entity, Position { x: 3.0, y: 4.0 } ));
    commands.spawn((Entity, Position { x: 4.0, y: 5.0 } ));
}

fn print_position(query: Query<&Position, With<Entity>>) {
    for position in &query {
        println!("position: {} {}", position.x, position.y);
    }
}

fn change_position(mut query: Query<&mut Position, With<Entity>>) {
    for mut pos in &mut query {
        pos.x += 1.0;
        pos.y += 1.0;
    }
}