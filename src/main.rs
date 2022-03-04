use rand::{Rng, SeedableRng};

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*, 
    window::{WindowMode, WindowResizeConstraints, WindowResized},
    ui::Val::Px, app::Events,
};

use heron::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgba(0.0, 0.0, 0.0, 1.0)))
        .insert_resource(
            WindowDescriptor {
                transparent: false,
                decorations: true,
                mode: WindowMode::Windowed,
                title: "Earth Escape".to_string(),
                width: 1200.,
                height: 800.,
                resize_constraints: WindowResizeConstraints {
                    min_height: 400.0,
                    min_width: 400.0,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_startup_system(add_player)
        .add_system(player_movement)
        .add_system(move_chasing_enemies)
        .add_system(spawn_chasers)
        // .add_system(text_update_system)
        .add_system(toggle_physics_pause)
        .add_system(fullscreen_toggle)
        .add_system(calculate_health)
        .add_system(reset_game)
        .add_system(resize_items)
        .add_system(increase_spawn_size)
        //.add_system(text_color_system)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ChasingEnemy;

#[derive(Component)]
struct Speed(f32);

// The u8 represents the placement of the heart
#[derive(Component)]
struct HeartSprite(u8);

// Define your physics layers
// Probably only need one or none
#[derive(PhysicsLayer)]
enum Layer {
    World,
    Player,
    Enemies,
}

fn resize_items(
    resize_event: Res<Events<WindowResized>>,
    mut player_query: Query<(&mut Sprite, &mut CollisionShape), (With<Player>, Without<ChasingEnemy>)>,
    mut chaser_query: Query<(&mut Sprite, &mut CollisionShape, &SizeScale), (With<ChasingEnemy>, Without<Player>)>,
) {
    let mut reader = resize_event.get_reader();
    for e in reader.iter(&resize_event) {
        let player_size = e.width / 20.;
        let (mut sprite, mut shape, ) = player_query.single_mut();
        sprite.custom_size = Some(Vec2::new(player_size, player_size));
        *shape =
            // CollisionShape::Cuboid {
            //     half_extends: Vec3::new(player_size / 2., player_size / 2., 0.0),
            //     border_radius: None,
            // };
            CollisionShape::Sphere {
                radius: player_size / 2.,
            };
        
        for (mut sprite, mut shape, SizeScale(size_scale)) in chaser_query.iter_mut() {
            let chaser_size = e.width / 40.;
            sprite.custom_size = Some(Vec2::new(chaser_size * size_scale, chaser_size * size_scale));
            *shape =
                // CollisionShape::Cuboid {
                //     half_extends: Vec3::new(chaser_size / 2., chaser_size / 2., 0.0),
                //     border_radius: None,
                // };
                CollisionShape::Sphere {
                    radius: (chaser_size * size_scale) / 2.,
                };
        }
    }
}

fn add_player(mut commands: Commands, windows: Res<Windows>, asset_server: Res<AssetServer>) {

    let size = windows.get_primary().unwrap().width() / 20.;

    commands
        .spawn_bundle(
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(size, size)),
                    ..Default::default()
                },
                texture: asset_server.load("sprites/PlayerEarth.png"),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..Default::default()
            }
        )
        .insert(Player)
        .insert(Speed(5.0))
        .insert(RigidBody::Dynamic)
        
        // Attach a collision shape
        // .insert(CollisionShape::Cuboid {
        //     half_extends: Vec3::new(size / 2., size / 2., 0.0),
        //     border_radius: None,
        // })
        .insert(CollisionShape::Sphere {
            radius: size / 2.,
        })

        
        
        // Optionally add other useful components...
        .insert(Velocity::default())
        // .insert(Acceleration::from_linear(Vec3::X * 1.0))
        .insert(PhysicMaterial { friction: 1.0, density: 20.0, ..Default::default() })
        .insert(Damping::from_linear(0.5).with_angular(1.0))
        .insert(RotationConstraints::lock())
        .insert(CollisionLayers::new(Layer::Player, Layer::Enemies))
        .insert(PlayerHealth(5));
}

struct SpawnTimer(Timer);

struct PlayerDied(bool);

#[derive(Component)]
struct SizeScale(f32);

fn spawn_chasers(
    mut commands: Commands,
    mut timer: ResMut<SpawnTimer>,
    time: Res<Time>,
    mut chaser_count: ResMut<ChaserCount>,
    mut enemy_count_text_query: Query<&mut Text, With<EnemyCountText>>,
    windows: Res<Windows>,
    chaser_sprite: Res<ChaserSprite>,
    chicken_sprite: Res<ChickenSprite>,
    mut random_gen: ResMut<RandomGenerator>,
    player_query: Query<&Transform, With<Player>>,
    player_died: Res<PlayerDied>,
    game_paused: ResMut<GamePaused>,
    size_increments: Res<SpawnSizeIncrements>,
) {
    if !game_paused.0 && !player_died.0 && timer.0.tick(time.delta()).just_finished() && !chaser_count.at_max() {

        let window_width = windows.get_primary().unwrap().width();
        let window_height = windows.get_primary().unwrap().height();

        let size = window_width / 40.;

        let player_transform = player_query.single().translation;

        let size_scale =
            if random_gen.0.gen_bool(0.75) { 
                random_gen.0.gen_range(0.8..1.2)
            } else { 
                random_gen.0.gen_range(0.75..2.5 + (size_increments.0 as f32 / 50.))
            };

        let spawn_x: f32 = 
            if random_gen.0.gen_bool(0.5) { 
                random_gen.0.gen_range((player_transform.x + window_width)..(player_transform.x + window_width + 100.)) 
            } else { 
                random_gen.0.gen_range((player_transform.x - window_width - 100.)..(player_transform.x - window_width)) 
            };

        let spawn_y: f32 = 
            if random_gen.0.gen_bool(0.5) { 
                random_gen.0.gen_range((player_transform.y + window_height)..(player_transform.y + window_height + 100.)) 
            } else {
                random_gen.0.gen_range((player_transform.y - window_height - 100.)..(player_transform.y - window_height)) 
            };

        commands
            .spawn_bundle(
                SpriteBundle {
                    texture: if random_gen.0.gen_bool(0.01) { chicken_sprite.0.clone() } else { chaser_sprite.0.clone() },
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(size * size_scale, size * size_scale)),
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(spawn_x, spawn_y, 0.0),
                    ..Default::default()
                }
            )
            .insert(ChasingEnemy)
            .insert(Speed(2.5))
            .insert(RigidBody::Dynamic)
            .insert(SizeScale(size_scale))
                    
            // Attach a collision shape
            // .insert(CollisionShape::Cuboid {
            //     half_extends: Vec3::new(size / 2., size / 2., 0.0),
            //     border_radius: None,
            // })
            .insert(CollisionShape::Sphere {
                radius: (size * size_scale) / 2.,
            })

            // // Optionally add other useful components...
            .insert(Velocity::default())
            // .insert(Velocity::from_linear(Vec3::X * 2.0))
            // .insert(Acceleration::from_linear(Vec3::X * -1.0))
            .insert(PhysicMaterial { friction: 1.0, density: 10.0 * size_scale, ..Default::default() })
            //.insert(RotationConstraints::lock())
            .insert(CollisionLayers::new(Layer::Enemies, Layer::Player).with_mask(Layer::Enemies));
            
            chaser_count.current += 1;
            enemy_count_text_query.single_mut().sections[1].style.color = Color::Rgba {
                red: 1.,
                green: (255. - chaser_count.current as f32) / 255.,
                blue: (255. - chaser_count.current as f32) / 255.,
                alpha: 1.
            };
            enemy_count_text_query.single_mut().sections[1].value = format!("{:.2}", chaser_count.current);
    }
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct EnemyCountText;

#[derive(Component)]
struct CenterMessageText;

#[derive(Component)]
struct SubCenterText;

// A unit struct to help identify the color-changing Text component
#[derive(Component)]
struct ColorText;

fn fullscreen_toggle(keyboard_input: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    if keyboard_input.just_pressed(KeyCode::F11) {
        let window = windows.get_primary_mut().unwrap();
        
        window.set_mode(
            match window.mode() {
                WindowMode::BorderlessFullscreen => WindowMode::Windowed,
                _ => WindowMode::BorderlessFullscreen,
            }
        );
    }
}

fn player_movement(
    game_paused: Res<GamePaused>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Transform, &Speed, &mut Velocity), (With<Player>, Without<Camera2D>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2D>, Without<Player>)>,
    player_died: Res<PlayerDied>,
    // mut touches: EventReader<TouchInput>,
    // windows: Res<Windows>,
) 
{
    if !game_paused.0 {
        let (transform, Speed(speed), mut velocity) = query.single_mut();

        camera_query.single_mut().translation = transform.translation;

        if !player_died.0 {
            let mut x = 0.0;
            let mut y = 0.0;

            // if let Some(touch) = touches.iter().next() {
            //     let window_width = windows.get_primary().unwrap().width();
            //     x += (touch.position.x - window_width / 2.) / (window_width / 2.);
            //     y += (touch.position.y - window_width / 2.) / (window_width / 2.);
            // } else {
            if keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left) {
                x -= 1.0;
            };
            if keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right) {
                x += 1.0;
            };
            if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
                y += 1.0;
            };
            if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
                y -= 1.0;
            };
            // }

            velocity.linear.x += x * speed;
            velocity.linear.y += y * speed;

            // transform.translation.x += x * speed;
            // transform.translation.y += y * speed;
        }
    }
}

fn move_chasing_enemies(
    game_paused: Res<GamePaused>,
    mut query: Query<(&Transform, &Speed, &mut Velocity), With<ChasingEnemy>>,
    player_query: Query<&Transform, (With<Player>, Without<ChasingEnemy>)>,
)
{
    if !game_paused.0 {
        if let Some(player_transform) = player_query.iter().next() {
            for (transform, Speed(speed), mut velocity) in query.iter_mut() {
                if transform.translation.x > player_transform.translation.x {
                    velocity.linear.x -= speed;
                } else {
                    velocity.linear.x += speed;
                }

                if transform.translation.y > player_transform.translation.y {
                    velocity.linear.y -= speed;
                } else {
                    velocity.linear.y += speed;
                }
            }
        }
    }
}

#[derive(Component)]
struct PlayerHealth(u8);

fn calculate_health(
    mut events: EventReader<CollisionEvent>,
    mut player_died: ResMut<PlayerDied>,
    mut heart_query: Query<(&mut UiImage, &HeartSprite), Without<PlayerHealth>>,
    full_heart_sprite: Res<FullHeartSprite>,
    empty_heart_sprite: Res<EmptyHeartSprite>,
    mut health_query: Query<&mut PlayerHealth>,
    mut center_text: Query<&mut Text, With<CenterMessageText>>,
    mut sub_center_text: Query<&mut Text, (With<SubCenterText>, Without<CenterMessageText>)>,
    mut enemy_spawn_timer: ResMut<SpawnTimer>,
) 
{
    if !player_died.0 {
        let mut health = health_query.single_mut();

        events
            .iter()
            .for_each(|event| {

                if event.is_stopped() {
                    let (layers_1, layers_2) = event.collision_layers();
                    if health.0 < 5 {
                        if is_player(layers_1) && is_enemy(layers_2) {
                            health.0 += 1;
                        } else if is_player(layers_2) && is_enemy(layers_1) {
                            health.0 += 1;
                        }
                    }
                }
                if event.is_started() {
                    if health.0 > 0 {
                        let (layers_1, layers_2) = event.collision_layers();
                        if is_player(layers_1) && is_enemy(layers_2) {
                            health.0 -= 1;
                        } else if is_player(layers_2) && is_enemy(layers_1) {
                            health.0 -= 1;
                        }
                    }
                }
            });

        if health.0 <= 0 {
            player_died.0 = true;
            center_text.single_mut().sections[0].value = String::from("You Died");
            sub_center_text.single_mut().sections[0].value = String::from("Press R to restart");
            enemy_spawn_timer.0.pause();
            for (mut sprite, _) in heart_query.iter_mut() {
                sprite.0 = empty_heart_sprite.0.clone();
            }
        } else {
            for (mut sprite, HeartSprite(id)) in heart_query.iter_mut() {
                if health.0 > *id {
                    sprite.0 = full_heart_sprite.0.clone();
                } else {
                    sprite.0 = empty_heart_sprite.0.clone();
                }
            }
        }
    }
}

// Note: We check both layers each time to avoid a false-positive
// that can occur if an entity has the default (unconfigured) `CollisionLayers`
fn is_player(layers: CollisionLayers) -> bool {
    layers.contains_group(Layer::Player) && !layers.contains_group(Layer::Enemies)
}

fn is_enemy(layers: CollisionLayers) -> bool {
    !layers.contains_group(Layer::Player) && layers.contains_group(Layer::Enemies)
}

#[derive(Component)]
struct Camera2D;

struct GamePaused(bool);
struct ChaserCount {
    current: u32,
    max: u32,
}

impl ChaserCount {
    fn new(current: u32, max: u32) -> Self {
        ChaserCount {
            current,
            max,
        }
    }

    fn at_max(&self) -> bool {
        self.current >= self.max
    }
}

fn increase_spawn_size(
    mut increments: ResMut<SpawnSizeIncrements>,
    player_died: Res<PlayerDied>,
    game_paused: ResMut<GamePaused>,
    mut timer: ResMut<IncreaseSpawnSizeTimer>,
    time: Res<Time>,
) {
    if !game_paused.0 && !player_died.0 && timer.0.tick(time.delta()).just_finished() {
        if increments.0 < 100 {
            increments.0 += 1;
        }
    }
}

struct SpawnSizeIncrements(u8);
struct IncreaseSpawnSizeTimer(Timer);
struct FullHeartSprite(Handle<Image>);
struct EmptyHeartSprite(Handle<Image>);
struct ChaserSprite(Handle<Image>);
struct RandomGenerator(rand::rngs::StdRng);
struct ChickenSprite(Handle<Image>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(OrthographicCameraBundle::new_2d()).insert(Camera2D);
    commands.insert_resource(SpawnTimer(Timer::from_seconds(0.5, true)));
    commands.insert_resource(IncreaseSpawnSizeTimer(Timer::from_seconds(5.0, true)));
    commands.insert_resource(PhysicsTime::new(1.5));
    commands.insert_resource(GamePaused(false));
    commands.insert_resource(ChaserCount::new(0, 1000));
    commands.insert_resource(PlayerDied(false));
    commands.insert_resource(SpawnSizeIncrements(0));

    // Text with one section
    // commands
    //     .spawn_bundle(TextBundle {
    //         style: Style {
    //             align_self: AlignSelf::FlexEnd,
    //             position_type: PositionType::Absolute,
    //             position: Rect {
    //                 bottom: Val::Px(5.0),
    //                 right: Val::Px(15.0),
    //                 ..Default::default()
    //             },
    //             ..Default::default()
    //         },
    //         // Use the `Text::with_section` constructor
    //         text: Text::with_section(
    //             // Accepts a `String` or any type that converts into a `String`, such as `&str`
    //             "hello\nbevy!",
    //             TextStyle {
    //                 font: asset_server.load("fonts/bahnschrift.ttf"),
    //                 font_size: 100.0,
    //                 color: Color::WHITE,
    //             },
    //             // Note: You can use `Default::default()` in place of the `TextAlignment`
    //             TextAlignment {
    //                 horizontal: HorizontalAlign::Center,
    //                 ..Default::default()
    //             },
    //         ),
    //         ..Default::default()
    //     })
    //     .insert(ColorText);
    // Rich text with multiple sections

    let bold_font: Handle<Font> = asset_server.load("fonts/Fredoka/Fredoka-Bold.ttf");

    let full_heart_sprite: Handle<Image> = asset_server.load("sprites/full_heart.png");
    let empty_heart_sprite: Handle<Image> = asset_server.load("sprites/empty_heart.png");
    
    commands.insert_resource(ChaserSprite(asset_server.load("sprites/Meteor1.png")));
    commands.insert_resource(ChickenSprite(asset_server.load("sprites/Chicken.png")));

    commands.insert_resource(FullHeartSprite(full_heart_sprite.clone()));
    commands.insert_resource(EmptyHeartSprite(empty_heart_sprite.clone()));

    commands.insert_resource(RandomGenerator(rand::rngs::StdRng::from_entropy()));

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                padding: Rect {
                    bottom: Px(16.0),
                    right: Px(16.0),
                    left: Px(16.0),
                    ..Default::default()
                },
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            // parent
            //     .spawn_bundle(
            //         TextBundle {
            //         // Use `Text` directly
            //         text: Text {
            //             // Construct a `Vec` of `TextSection`s
            //             sections: vec![
            //                 TextSection {
            //                     value: "FPS: ".to_string(),
            //                     style: TextStyle {
            //                         font: bold_font.clone(),
            //                         font_size: 32.0,
            //                         color: Color::WHITE,
            //                     },
            //                 },
            //                 TextSection {
            //                     value: "".to_string(),
            //                     style: TextStyle {
            //                         font: bold_font.clone(),
            //                         font_size: 32.0,
            //                         color: Color::GOLD,
            //                     },
            //                 },
            //             ],
            //             ..Default::default()
            //         },
            //         ..Default::default()
            //     })
            //     .insert(FpsText);

            parent
                .spawn_bundle(NodeBundle {
                    color: Color::NONE.into(),
                    style: Style {
                        padding: Rect::all(Px(8.0)),                        
                        ..Default::default()
                    },
                    ..Default::default()
                }).with_children(|nested_parent| {
                    nested_parent
                        .spawn_bundle(TextBundle {
                            // Use `Text` directly
                            text: Text {
                                // Construct a `Vec` of `TextSection`s
                                sections: vec![
                                    TextSection {
                                        value: "Enemy Count: ".to_string(),
                                        style: TextStyle {
                                            font: bold_font.clone(),
                                            font_size: 48.0,
                                            color: Color::WHITE,
                                        },
        
                                    },
                                    TextSection {
                                        value: "0".to_string(),
                                        style: TextStyle {
                                            font: bold_font.clone(),
                                            font_size: 48.0,
                                            color: Color::WHITE,
                                        },
                                    },
                                ],
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(EnemyCountText);
                });                
        });

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::ColumnReverse,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(TextBundle {
                    // Use `Text` directly
                    text: Text {
                        // Construct a `Vec` of `TextSection`s
                        sections: vec![
                            TextSection {
                                value: "".to_string(),
                                style: TextStyle {
                                    font: bold_font.clone(),
                                    font_size: 82.0,
                                    color: Color::WHITE,
                                },
                            },
                        ],
                        ..Default::default()
                    },
                ..Default::default()
            })
            .insert(CenterMessageText);
            
                parent
                .spawn_bundle(TextBundle {
                    // Use `Text` directly
                    text: Text {
                        // Construct a `Vec` of `TextSection`s
                        sections: vec![
                            TextSection {
                                value: "".to_string(),
                                style: TextStyle {
                                    font: bold_font.clone(),
                                    font_size: 36.0,
                                    color: Color::GREEN,
                                },
                            },
                        ],
                        ..Default::default()
                    },
                ..Default::default()
            })
            .insert(SubCenterText);
        });

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                padding: Rect {
                    top: Px(16.0),
                    ..Default::default()
                },
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,

                        ..Default::default()
                    },
                    color: Color::NONE.into(),
                    ..Default::default()
                })
                .with_children(|nested_parent| {
                    nested_parent
                        .spawn_bundle(ImageBundle {
                            image: full_heart_sprite.clone().into(),
                            ..Default::default()
                        })
                        .insert(HeartSprite(0));
                    
                    nested_parent
                        .spawn_bundle(ImageBundle {
                            image: full_heart_sprite.clone().into(),
                            ..Default::default()
                        })
                        .insert(HeartSprite(1));

                    nested_parent
                        .spawn_bundle(ImageBundle {
                            image: full_heart_sprite.clone().into(),
                            ..Default::default()
                        })
                        .insert(HeartSprite(2));

                    nested_parent
                        .spawn_bundle(ImageBundle {
                            image: full_heart_sprite.clone().into(),
                            ..Default::default()
                        })
                        .insert(HeartSprite(3));

                    nested_parent
                        .spawn_bundle(ImageBundle {
                            image: full_heart_sprite.clone().into(),
                            ..Default::default()
                        })
                        .insert(HeartSprite(4));
                });
            
        });

    // commands
    //     .spawn_bundle(
    //         ImageBundle {
    //             style: Style {
    //                 // align_self: AlignSelf::FlexEnd,
    //                 // margin: Rect {
    //                 //     left: Px(8.0),
    //                 //     top: Px(8.0),
    //                 //     right: bevy::ui::Val::Auto,
    //                 //     ..Default::default()
    //                 // },
    //                 position_type: PositionType::Absolute,
    //                 position: Rect {
    //                     left: Px(8.0),
    //                     top: Px(8.0),
    //                     ..Default::default()
    //                 },

    //                 ..Default::default()
    //             },
    //             ..Default::default()
    //         },
    //     )
    //     .insert(FpsText);
}


// Need to add timers to this as they are added to the game.
// Also important. Need to check GamePaused flag in other systems before applying changes.
fn toggle_physics_pause(
    input: Res<Input<KeyCode>>,
    mut physics_time: ResMut<PhysicsTime>,
    mut game_paused: ResMut<GamePaused>,
    mut enemy_spawn_timer: ResMut<SpawnTimer>,
    mut center_text: Query<&mut Text, With<CenterMessageText>>,
    player_died: Res<PlayerDied>,
) {
    if !player_died.0 && input.just_pressed(KeyCode::Space) {
        if game_paused.0 {
            physics_time.resume();
            enemy_spawn_timer.0.unpause();
            game_paused.0 = false;
            center_text.single_mut().sections[0].value = "".to_string();
            
        } else {
            physics_time.pause();
            enemy_spawn_timer.0.pause();
            game_paused.0 = true;
            center_text.single_mut().sections[0].value = "Paused".to_string();
        }
    }
}

fn reset_game(
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    mut physics_time: ResMut<PhysicsTime>,
    mut game_paused: ResMut<GamePaused>,
    mut enemy_spawn_timer: ResMut<SpawnTimer>,
    mut center_text: Query<&mut Text, (With<CenterMessageText>, Without<SubCenterText>, Without<EnemyCountText>)>,
    mut sub_center_text: Query<&mut Text, (With<SubCenterText>, Without<CenterMessageText>, Without<EnemyCountText>)>,
    mut player_died: ResMut<PlayerDied>,
    mut health_query: Query<&mut PlayerHealth>,
    chaser_query: Query<Entity, With<ChasingEnemy>>,
    mut player_query: Query<(&mut Transform, &mut Velocity), With<Player>>,
    mut chaser_count: ResMut<ChaserCount>,
    mut enemy_count_text_query: Query<&mut Text, (With<EnemyCountText>, Without<CenterMessageText>, Without<SubCenterText>)>
) {
    if player_died.0 && input.just_pressed(KeyCode::R) {
        chaser_query.iter().for_each(|e| commands.entity(e).despawn());
        chaser_count.current = 0;
        enemy_count_text_query.single_mut().sections[1].value = String::from("0");
        let (mut transform, mut velocity) = player_query.single_mut();
        *transform = Transform::from_xyz(0.0, 0.0, 0.0);
        *velocity = Velocity::from_linear(Vec3::new(0.0, 0.0, 0.0));
        physics_time.resume();
        enemy_spawn_timer.0.reset();
        enemy_spawn_timer.0 = Timer::from_seconds(0.5, true);
        enemy_spawn_timer.0.unpause();
        game_paused.0 = false;
        center_text.single_mut().sections[0].value = String::from("");
        sub_center_text.single_mut().sections[0].value = String::from("");
        health_query.single_mut().0 = 5;
        player_died.0 = false;
    }
}

fn text_update_system(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}