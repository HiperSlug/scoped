use bevy::prelude::*;
use rand::seq::IndexedRandom;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(HelloPlugin)
        .run();
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_people)
            .add_systems(Update, (change_name, say_hello).chain())
            .insert_resource(GreetTimer(Timer::from_seconds(2.0, TimerMode::Repeating)));
    }
}

fn add_people(mut commands: Commands) {
    commands.spawn((
        Person,
        Name {
            title: Title::random(),
            name: String::from("Bill"),
        },
    ));
}

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name {
    title: Title,
    name: String,
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.title, self.name)
    }
}

#[derive(Clone, Copy)]
enum Title {
    Mr,
    Ms,
    Mrs,
    Lord,
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Mr => "Mr.",
            Self::Mrs => "Mrs.",
            Self::Lord => "Lord",
            Self::Ms => "Ms",
        };
        write!(f, "{s}")
    }
}

impl Title {
    fn random() -> Self {
        let choices = [Title::Mr, Title::Mrs, Title::Lord, Title::Ms];
        let mut rng = rand::rng();
        *choices.choose(&mut rng).unwrap()
    }
}

fn say_hello(time: Res<Time>, mut timer: ResMut<GreetTimer>, query: Query<&Name, With<Person>>) {
    if timer.0.tick(time.delta()).just_finished() {
        for name in &query {
            println!("Hello {name}!");
        }
    }
}

#[derive(Resource)]
struct GreetTimer(Timer);

fn change_name(mut query: Query<&mut Name, With<Person>>) {
    for mut name in &mut query {
        name.title = Title::random();
    }
}
