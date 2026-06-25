use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    sprite::Anchor,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GraphPlugin)
        .run();
}

/// Default pixel size of cells on grid.
const DEFAULT_CELL_SIZE: f32 = 170.0;

/// Damps the camera scale (zoom) speed. This value has been tuned to work on MacOS touchpad.
const CAMERA_SCALE_DAMPING: f32 = 0.005;

const GRID_BACKGROUND_COLOR: Color = Color::WHITE;
const GRID_AXIS_COLOR: Color = Color::BLACK;
const GRID_CELL_LINE_COLOR: LinearRgba = LinearRgba {
    red: 0.2,
    green: 0.2,
    blue: 0.2,
    alpha: 0.55,
};
const GRID_SUBCELL_LINE_COLOR: LinearRgba = LinearRgba {
    red: 0.2,
    green: 0.2,
    blue: 0.2,
    alpha: 0.35,
};
const ORIGIN_ZONE_PADDING: f32 = 40.0;

const TEXT_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.95);
const TEXT_COLOR_SUBTLE: Color = Color::srgba(0.1, 0.1, 0.1, 0.85);
const TEXT_FONT_SIZE: f32 = 15.0;
const TEXT_MARGIN: f32 = 5.0;

/// Pre-allocated pool size per axis. Supports up to this many visible labels at once.
/// At the minimum visual cell size (~85px) on a 2560px wide screen: ceil(1280/85)+1 ≈ 17.
/// 48 gives comfortable headroom for large monitors.
const MAX_LABELS_PER_AXIS: usize = 48;

#[derive(Component)]
struct XLabel;

#[derive(Component)]
struct YLabel;

/// Holds pre-spawned label entities so `draw_grid` can update them in-place
/// instead of despawning/spawning every frame.
#[derive(Resource)]
struct LabelPool {
    x: Vec<Entity>,
    y: Vec<Entity>,
}

struct GraphPlugin;

impl Plugin for GraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.insert_resource(ClearColor(GRID_BACKGROUND_COLOR));
        app.add_systems(
            Update,
            (draw_grid, handle_zoom_input, handle_pan_input),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let x = (0..MAX_LABELS_PER_AXIS)
        .map(|_| {
            commands
                .spawn((
                    XLabel,
                    Text2d::new(""),
                    TextFont {
                        font_size: FontSize::Px(TEXT_FONT_SIZE),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Anchor::TOP_CENTER,
                    Visibility::Hidden,
                    Transform::default(),
                ))
                .id()
        })
        .collect();

    let y = (0..MAX_LABELS_PER_AXIS)
        .map(|_| {
            commands
                .spawn((
                    YLabel,
                    Text2d::new(""),
                    TextFont {
                        font_size: FontSize::Px(TEXT_FONT_SIZE),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Anchor::CENTER_LEFT,
                    Visibility::Hidden,
                    Transform::default(),
                ))
                .id()
        })
        .collect();

    commands.insert_resource(LabelPool { x, y });
}

fn handle_zoom_input(
    mut gizmos: Gizmos,
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    window: Single<&Window>,
    mut camera_transform: Single<&mut Transform, With<Camera2d>>,
) {
    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };

    // Cursor offset from viewport center in world-space direction (flip y: screen y-down → world y-up)
    let viewport_center = Vec2::new(window.width(), window.height()) / 2.0;
    let cursor_from_center = Vec2::new(
        cursor_screen.x - viewport_center.x,
        -(cursor_screen.y - viewport_center.y),
    );

    let camera_scale = camera_transform.scale.x;
    let camera_pos = camera_transform.translation.xy();
    let cursor_world = camera_pos + cursor_from_center * camera_scale;

    let current_padding = ORIGIN_ZONE_PADDING * camera_scale;
    let scale_transform =
        if is_within_zone(cursor_world, Vec2::ZERO - current_padding, Vec2::ZERO + current_padding) {
            -camera_pos / camera_scale
        } else {
            cursor_from_center
        };

    for event in mouse_wheel_events.read() {
        let old_scale = camera_transform.scale.x;
        // Scroll up (event.y > 0) → zoom in → smaller scale (camera sees fewer world units)
        let new_scale = old_scale / (1.0 + event.y * CAMERA_SCALE_DAMPING);
        // Keep the world point under the cursor fixed
        let scale_delta = old_scale - new_scale;
        camera_transform.translation.x += scale_transform.x * scale_delta;
        camera_transform.translation.y += scale_transform.y * scale_delta;
        camera_transform.scale = Vec3::new(new_scale, new_scale, 1.0);
    }

    #[cfg(debug_assertions)]
    {
        let debug_box_size = ORIGIN_ZONE_PADDING * 2.0 * camera_scale;
        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::ZERO),
            Vec2::splat(debug_box_size),
            Color::linear_rgb(0.0, 1.0, 0.0),
        );
    }
}

fn handle_pan_input(
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_transform: Single<&mut Transform, With<Camera2d>>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    let scale = camera_transform.scale.x;
    for motion in mouse_motion_events.read() {
        // Scale delta by camera zoom so screen-pixel movement feels consistent at any zoom level
        camera_transform.translation.x -= motion.delta.x * scale;
        camera_transform.translation.y += motion.delta.y * scale;
    }
}

/// Renders the infinite grid (gizmos) and updates axis label positions/values (pre-allocated pool).
/// Both use the same derived layout values so calculations run once per frame.
fn draw_grid(
    mut gizmos: Gizmos,
    pool: Res<LabelPool>,
    window: Single<&Window>,
    camera_transform: Single<&Transform, With<Camera2d>>,
    mut x_query: Query<
        (&mut Text2d, &mut Transform, &mut Visibility, &mut TextFont),
        (With<XLabel>, Without<YLabel>, Without<Camera2d>),
    >,
    mut y_query: Query<
        (&mut Text2d, &mut Transform, &mut Visibility, &mut TextFont, &mut Anchor),
        (With<YLabel>, Without<XLabel>, Without<Camera2d>),
    >,
) {
    // --- Shared layout (computed once) ---
    let camera_scale = camera_transform.scale.x;
    let cell_scale = halve_step(camera_scale);
    let cell_size = DEFAULT_CELL_SIZE * cell_scale;
    let num_subunits = subdivision_count(cell_size);
    let subcell_size = cell_size / num_subunits as f32;

    let cam_x = camera_transform.translation.x;
    let cam_y = camera_transform.translation.y;
    let world_width = window.width() * camera_scale;
    let world_height = window.height() * camera_scale;

    let snapped_x = (cam_x / cell_size).round() * cell_size;
    let snapped_y = (cam_y / cell_size).round() * cell_size;
    let half_cols = ((world_width / 2.0) / cell_size).ceil() as i32 + 1;
    let half_rows = ((world_height / 2.0) / cell_size).ceil() as i32 + 1;
    let cols = (half_cols as u32 * 2).max(2);
    let rows = (half_rows as u32 * 2).max(2);

    // --- Grid lines ---
    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::new(snapped_x, snapped_y)),
        UVec2::new(cols, rows),
        Vec2::new(cell_size, cell_size),
        GRID_CELL_LINE_COLOR,
    );
    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::new(snapped_x, snapped_y)),
        UVec2::new(cols * num_subunits, rows * num_subunits),
        Vec2::new(subcell_size, subcell_size),
        GRID_SUBCELL_LINE_COLOR,
    );
    gizmos.line_2d(
        Vec2::new(cam_x - world_width, 0.0),
        Vec2::new(cam_x + world_width, 0.0),
        GRID_AXIS_COLOR,
    );
    gizmos.line_2d(
        Vec2::new(0.0, cam_y - world_height),
        Vec2::new(0.0, cam_y + world_height),
        GRID_AXIS_COLOR,
    );

    // --- Label pool update (in-place, no entity churn) ---
    let font_size = TEXT_FONT_SIZE * camera_scale;
    let margin = TEXT_MARGIN * camera_scale;

    let x_label_y = (0f32 - margin * 0.5).clamp(
        cam_y - world_height / 2.0 + font_size + margin,
        cam_y + world_height / 2.0 - margin,
    );
    let left_edge = cam_x - world_width / 2.0;
    let right_edge = cam_x + world_width / 2.0;
    let y_axis_x = margin; // desired: just right of y-axis (world x = 0)
    let (y_label_x, y_label_anchor) = if y_axis_x < left_edge + margin {
        // y-axis off-screen to the left: pin to left edge, text extends right
        (left_edge + margin, Anchor::CENTER_LEFT)
    } else if y_axis_x > right_edge - margin {
        // y-axis off-screen to the right: pin to right edge, text extends left
        (right_edge - margin, Anchor::CENTER_RIGHT)
    } else {
        (y_axis_x, Anchor::CENTER_LEFT)
    };

    let mut xi = 0usize;
    for i in -half_cols..=half_cols {
        let x = snapped_x + i as f32 * cell_size;
        let value = x / DEFAULT_CELL_SIZE;
        if value.abs() < 1e-4 {
            continue;
        }
        if xi >= pool.x.len() {
            break;
        }
        if let Ok((mut text, mut tfm, mut vis, mut font)) = x_query.get_mut(pool.x[xi]) {
            *text = Text2d::new(format_label(value, cell_scale));
            tfm.translation = Vec3::new(x, x_label_y, 1.0);
            font.font_size = FontSize::Px(font_size);
            *vis = Visibility::Visible;
        }
        xi += 1;
    }
    for idx in xi..pool.x.len() {
        if let Ok((_, _, mut vis, _)) = x_query.get_mut(pool.x[idx]) {
            *vis = Visibility::Hidden;
        }
    }

    let mut yi = 0usize;
    for j in -half_rows..=half_rows {
        let y = snapped_y + j as f32 * cell_size;
        let value = y / DEFAULT_CELL_SIZE;
        if value.abs() < 1e-4 {
            continue;
        }
        if yi >= pool.y.len() {
            break;
        }
        if let Ok((mut text, mut tfm, mut vis, mut font, mut anchor)) = y_query.get_mut(pool.y[yi]) {
            *text = Text2d::new(format_label(value, cell_scale));
            tfm.translation = Vec3::new(y_label_x, y, 1.0);
            font.font_size = FontSize::Px(font_size);
            *anchor = y_label_anchor;
            *vis = Visibility::Visible;
        }
        yi += 1;
    }
    for idx in yi..pool.y.len() {
        if let Ok((_, _, mut vis, _, _)) = y_query.get_mut(pool.y[idx]) {
            *vis = Visibility::Hidden;
        }
    }
}

fn is_within_zone(point: Vec2, zone_min: Vec2, zone_max: Vec2) -> bool {
    (point.cmpge(zone_min) & point.cmple(zone_max)).all()
}

/// A step function that halves every time x halves
fn halve_step(x: f32) -> f32 {
    x.log2().floor().exp2()
}

fn subdivision_count(cell_world: f32) -> u32 {
    let exp = cell_world.log10().floor();
    let mantissa = (cell_world / 10f32.powf(exp)).round() as u32;
    match mantissa {
        1 | 5 => 5,
        2 => 4,
        _ => 5,
    }
}

/// Formats a coordinate value with precision matched to the current cell scale.
/// At cell_scale >= 1 shows integers; at fractions shows the right number of decimals.
fn format_label(value: f32, cell_scale: f32) -> String {
    let decimals = if cell_scale >= 1.0 {
        0
    } else {
        cell_scale.log2().abs().ceil() as usize
    };
    let s = format!("{:.prec$}", value, prec = decimals);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}
