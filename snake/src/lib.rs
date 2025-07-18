use bevy::prelude::*;

pub struct SnakeGame;

impl Plugin for SnakeGame {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Snake"),
                resolution: (500.0, 500.0).into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .insert_resource(ClearColor(Color::srgba(0.1, 0.1, 0.1, 1.0)))
        .add_systems(Startup, (setup, snake::setup))
        .add_systems(
            PostUpdate,
            (space::position_translation, space::size_scaling),
        )
        .add_systems(Update, snake::input)
        .add_systems(PreUpdate, move_tick)
        .add_systems(
            Update,
            (
                snake::movement,
                food::eat,
                (snake::grow, food::spawn_sys).run_if(food::ate),
            )
                .chain()
                .run_if(move_finished),
        )
        .insert_resource(snake::SnakeBits::default())
        .add_event::<food::EatEvent>()
        .insert_resource(MoveTimer(Timer::from_seconds(0.15, TimerMode::Repeating)))
        .insert_resource(snake::LastTailPosition::default())
        .add_event::<GameOverEvent>()
        .add_systems(PostStartup, food::spawn_sys)
        .add_systems(PostUpdate, game_over);
    }
}

fn setup(mut cmd: Commands) {
    cmd.spawn(Camera2d);
}

#[derive(Resource)]
struct MoveTimer(pub Timer);

fn move_tick(mut timer: ResMut<MoveTimer>, time: Res<Time>) {
    timer.0.tick(time.delta());
}

fn move_finished(timer: Res<MoveTimer>) -> bool {
    timer.0.just_finished()
}

#[derive(Event)]
pub struct GameOverEvent;

fn game_over(
    mut reader: EventReader<GameOverEvent>,
    mut cmd: Commands,
    mut snake_bits: ResMut<snake::SnakeBits>,
    food: Query<Entity, With<food::Food>>,
    snake: Query<Entity, With<snake::SnakeBit>>,
) {
    if reader.read().next().is_some() {
        for e in food.iter().chain(snake.iter()) {
            cmd.entity(e).despawn();
        }

        snake_bits.0.clear();

        snake::setup(cmd.reborrow(), snake_bits);
        food::spawn(
            cmd,
            space::Position::random_exceptions(&[
                snake::STARTING_HEAD_POSITION,
                snake::STARTING_HEAD_POSITION - space::Position { x: 0, y: 1 },
            ]),
        );
    }
}

pub mod space {
    use std::ops::Sub;

    use bevy::prelude::*;
    use bevy::window::PrimaryWindow;
    use rand::random;

    pub const ARENA_WIDTH: u32 = 10;
    pub const ARENA_HEIGHT: u32 = 10;

    #[derive(Component, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Position {
        pub x: i32,
        pub y: i32,
    }

    impl Position {
        pub fn random() -> Self {
            Self {
                x: { random::<u32>() % ARENA_WIDTH } as i32,
                y: { random::<u32>() % ARENA_HEIGHT } as i32,
            }
        }

        pub fn random_exceptions(exceptions: &[Position]) -> Self {
            loop {
                let r = Self::random();
                if !exceptions.contains(&r) {
                    break r;
                }
            }
        }
    }

    impl Sub for Position {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self::Output {
            Self {
                x: self.x - rhs.x,
                y: self.y - rhs.y,
            }
        }
    }

    pub fn position_translation(
        window: Query<&Window, With<PrimaryWindow>>,
        mut q: Query<(&Position, &mut Transform)>,
    ) {
        fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
            let tile_size = bound_window / bound_game;
            pos / bound_game * bound_window - (bound_window / 2.0) + (tile_size / 2.0)
        }

        let w = window.single().unwrap();

        for (pos, mut transform) in q.iter_mut() {
            transform.translation = Vec3::new(
                convert(pos.x as f32, w.width(), ARENA_WIDTH as f32),
                convert(pos.y as f32, w.height(), ARENA_HEIGHT as f32),
                0.0,
            );
        }
    }

    #[derive(Component)]
    pub struct Size {
        pub width: f32,
        pub height: f32,
    }

    impl Size {
        pub fn square(x: f32) -> Self {
            Self {
                width: x,
                height: x,
            }
        }
    }

    pub fn size_scaling(
        window: Query<&Window, With<PrimaryWindow>>,
        mut q: Query<(&Size, &mut Transform)>,
    ) {
        let w = window.single().unwrap();
        for (size, mut transform) in q.iter_mut() {
            transform.scale = Vec3::new(
                size.width / ARENA_WIDTH as f32 * w.width(),
                size.height / ARENA_HEIGHT as f32 * w.height(),
                1.0,
            );
        }
    }
}

pub mod snake {
    use super::GameOverEvent;
    use super::space::*;
    use bevy::prelude::*;

    const SNAKE_HEAD_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
    const SNAKE_BODY_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);

    pub const STARTING_HEAD_POSITION: Position = Position { x: 5, y: 2 };

    #[derive(Component)]
    pub struct SnakeBit;

    #[derive(Default, Resource)]
    pub struct SnakeBits(pub Vec<Entity>);

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    enum Direction {
        Up,
        Down,
        Left,
        Right,
    }

    impl Direction {
        pub fn opposite(self) -> Self {
            match self {
                Self::Down => Self::Up,
                Self::Right => Self::Left,
                Self::Left => Self::Right,
                Self::Up => Self::Down,
            }
        }
    }

    #[derive(Component, Debug)]
    pub struct SnakeHead {
        dir: Direction,
        turned: bool, // flag preventing multiple turns in one tick.
    }

    impl SnakeHead {
        fn try_dir(&mut self, dir: Direction) {
            if dir.opposite() == self.dir {
                return;
            };
            self.dir = dir;
            self.turned = true;
        }
    }

    #[derive(Resource, Default)]
    pub struct LastTailPosition(Option<Position>);

    pub fn setup(mut cmd: Commands, mut snake_bits: ResMut<SnakeBits>) {
        let head = cmd
            .spawn((
                Sprite::from_color(SNAKE_HEAD_COLOR, Vec2::ONE),
                Transform::default(),
                SnakeHead {
                    dir: Direction::Up,
                    turned: false,
                },
                STARTING_HEAD_POSITION,
                Size::square(0.8),
                SnakeBit,
            ))
            .id();
        snake_bits.0.push(head);

        snake_bits
            .0
            .push(spawn(cmd, STARTING_HEAD_POSITION - Position { x: 0, y: 1 }));
    }

    pub fn spawn(mut cmd: Commands, position: Position) -> Entity {
        cmd.spawn((
            Sprite::from_color(SNAKE_BODY_COLOR, Vec2::ONE),
            Transform::default(),
            SnakeBit,
            position,
            Size::square(0.65),
        ))
        .id()
    }

    pub fn input(input: Res<ButtonInput<KeyCode>>, mut q: Query<&mut SnakeHead>) {
        let mut head = q.single_mut().unwrap();

        if head.turned {
            return;
        }

        if input.pressed(KeyCode::KeyD) {
            head.try_dir(Direction::Right);
        } else if input.pressed(KeyCode::KeyS) {
            head.try_dir(Direction::Down);
        } else if input.pressed(KeyCode::KeyA) {
            head.try_dir(Direction::Left);
        } else if input.pressed(KeyCode::KeyW) {
            head.try_dir(Direction::Up);
        };
    }

    pub fn movement(
        mut q: Query<(Entity, &mut SnakeHead)>,
        mut positions: Query<&mut Position>,
        snake_bits: Res<SnakeBits>,
        mut last_tail_position: ResMut<LastTailPosition>,
        mut game_over: EventWriter<GameOverEvent>,
    ) {
        let (head_e, mut head) = q.single_mut().unwrap();

        head.turned = false;

        let bit_pos = snake_bits
            .0
            .iter()
            .map(|e| *positions.get(*e).unwrap())
            .collect::<Vec<Position>>();

        *last_tail_position = LastTailPosition(Some(*bit_pos.last().unwrap()));

        let mut head_pos = positions.get_mut(head_e).unwrap();

        match &head.dir {
            Direction::Right => head_pos.x += 1,
            Direction::Down => head_pos.y -= 1,
            Direction::Left => head_pos.x -= 1,
            Direction::Up => head_pos.y += 1,
        };

        if head_pos.x < 0
            || head_pos.x > ARENA_WIDTH as i32 - 1
            || head_pos.y < 0
            || head_pos.y > ARENA_HEIGHT as i32 - 1
        {
            game_over.write(GameOverEvent);
        }

        let head_pos = head_pos.clone();

        snake_bits
            .0
            .iter()
            .skip(1)
            .zip(bit_pos.iter())
            .for_each(|(e, pos)| {
                *positions.get_mut(*e).unwrap() = *pos;
                if *pos == head_pos {
                    game_over.write(GameOverEvent);
                }
            });
    }

    pub fn grow(
        cmd: Commands,
        last_tail_position: Res<LastTailPosition>,
        mut snake_bits: ResMut<SnakeBits>,
    ) {
        snake_bits.0.push(spawn(cmd, last_tail_position.0.unwrap()));
    }
}

pub mod food {
    use crate::snake::SnakeBit;

    use super::snake::SnakeHead;
    use super::space::*;
    use bevy::prelude::*;

    const FOOD_COLOR: Color = Color::srgb(1.0, 0.5, 1.0);

    #[derive(Component)]
    pub struct Food;

    #[derive(Event)]
    pub struct EatEvent;

    pub fn spawn_sys(cmd: Commands, snake_positions: Query<&Position, With<SnakeBit>>) {
        let positions = snake_positions
            .iter()
            .map(|s| s.clone())
            .collect::<Vec<Position>>();

        let pos = Position::random_exceptions(&positions);

        spawn(cmd, pos);
    }

    pub fn spawn(mut cmd: Commands, position: Position) {
        cmd.spawn((
            Sprite::from_color(FOOD_COLOR, Vec2::ONE),
            Food,
            position,
            Transform::default(),
            Size::square(0.8),
        ));
    }

    pub fn eat(
        mut cmd: Commands,
        mut eat_writer: EventWriter<EatEvent>,
        food_positions: Query<(Entity, &Position), With<Food>>,
        head_position: Query<&Position, With<SnakeHead>>,
    ) {
        let head_pos = head_position.single().unwrap();

        for (e, pos) in food_positions {
            if pos == head_pos {
                cmd.entity(e).despawn();
                eat_writer.write(EatEvent);
            }
        }
    }

    pub fn ate(mut eat_reader: EventReader<EatEvent>) -> bool {
        eat_reader.read().next().is_some()
    }
}
