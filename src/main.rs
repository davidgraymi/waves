use std::ops::Range;

use bevy::{
    camera,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GraphPlugin)
        .run();
}

/// Default pixel size of cells on grid.
/// TODO: make a min and max size for log scaling
const DEFAULT_CELL_SIZE: f32 = 170.0;
// const MIN_CELL_SIZE: f32 = 84.0;
// const MAX_CELL_SIZE: f32 = MIN_CELL_SIZE * 2.0;

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

const CENTER_CURSOR_ZONE_MAX: Vec2 = Vec2 { x: 30.0, y: 30.0 };
const CENTER_CURSOR_ZONE_MIN: Vec2 = Vec2 { x: -30.0, y: -30.0 };

struct GraphPlugin;

impl Plugin for GraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.insert_resource(ClearColor(GRID_BACKGROUND_COLOR));
        app.add_systems(
            Update,
            (draw_infinite_grid, handle_zoom_input, handle_pan_input),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn handle_zoom_input(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    window: Single<&Window>,
    mut camera_transform: Single<&mut Transform, With<Camera2d>>,
) {
    let Some(cursor_screen) = window.cursor_position() else {
        return;
    };

    // TODO: change from screen center to grid (0,0)
    // Cursor offset from viewport center in world-space direction (flip y: screen y-down → world y-up)
    let viewport_center = Vec2::new(window.width(), window.height()) / 2.0;
    let cursor_from_center = Vec2::new(
        cursor_screen.x - viewport_center.x,
        -(cursor_screen.y - viewport_center.y),
    );
    let scale_transform = if is_within_center_zone(cursor_from_center) {
        println!("cursor within center");
        Vec2 { x: 0.0, y: 0.0 }
    } else {
        println!("cursor NOT within center");
        cursor_from_center
    };

    for event in mouse_wheel_events.read() {
        let old_scale = camera_transform.scale.x;
        // Scroll up (event.y > 0) → zoom in → smaller scale (camera sees fewer world units)
        let new_scale = (old_scale / (1.0 + event.y * CAMERA_SCALE_DAMPING));

        // Keep the world point under the cursor fixed:
        // world_cursor = camera_pos + cursor_from_center * old_scale
        // new_camera_pos = world_cursor - cursor_from_center * new_scale
        //                = camera_pos + cursor_from_center * (old_scale - new_scale)
        let scale_delta = old_scale - new_scale;
        camera_transform.translation.x += scale_transform.x * scale_delta;
        camera_transform.translation.y += scale_transform.y * scale_delta;
        camera_transform.scale = Vec3::new(new_scale, new_scale, 1.0);
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

/// Renders a coordinate grid that tracks the camera position endlessly
fn draw_infinite_grid(
    mut gizmos: Gizmos,
    window: Single<&Window>,
    camera_transform: Single<&Transform, With<Camera2d>>,
) {
    let viewport_width = window.width();
    let viewport_height = window.height();
    let camera_scale = camera_transform.scale.x;

    // Calculate the cell size dynamically
    let cell_scale = halve_step(camera_scale);
    let cell_size = DEFAULT_CELL_SIZE * cell_scale;
    let num_subunits: u32 = 4;
    let subcell_size = cell_size / num_subunits as f32;

    // World units visible through the viewport at the current zoom level
    let world_width = to_world_units(viewport_width, camera_scale);
    let world_height = to_world_units(viewport_height, camera_scale);

    let half_cols = ((world_width / 2.0) / cell_size).ceil() as u32 + 1;
    let half_rows = ((world_height / 2.0) / cell_size).ceil() as u32 + 1;
    let cols = (half_cols * 2).max(2);
    let rows = (half_rows * 2).max(2);

    let cam_x = camera_transform.translation.x;
    let cam_y = camera_transform.translation.y;
    let snapped_x = (cam_x / cell_size).round() * cell_size;
    let snapped_y = (cam_y / cell_size).round() * cell_size;

    // Unit grid
    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::new(snapped_x, snapped_y)),
        UVec2::new(cols, rows),
        Vec2::new(cell_size, cell_size),
        GRID_CELL_LINE_COLOR,
    );

    // Subunit grid
    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::new(snapped_x, snapped_y)),
        UVec2::new(cols * num_subunits, rows * num_subunits),
        Vec2::new(subcell_size, subcell_size),
        GRID_SUBCELL_LINE_COLOR,
    );

    // Bold primary axes spanning the visible world area
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
}

fn is_within_center_zone(point: Vec2) -> bool {
    (point.cmpge(CENTER_CURSOR_ZONE_MIN) & point.cmple(CENTER_CURSOR_ZONE_MAX)).all()
}

fn to_world_units(pixels: f32, scale: f32) -> f32 {
    pixels * scale
}

/// A step function that halves every time x halves
fn halve_step(x: f32) -> f32 {
    x.log2().floor().exp2()
}

fn next_nice_number(min_world: f32) -> f32 {
    let exp = min_world.log10().floor();
    let base = 10f32.powf(exp);
    // try 1×, 2×, 5× of the decade
    for &factor in &[1.0f32, 2.0, 5.0] {
        if base * factor >= min_world {
            return base * factor;
        }
    }
    base * 10.0
}

fn subdivision_count(large_world: f32) -> u32 {
    let exp = large_world.log10().floor();
    let mantissa = (large_world / 10f32.powf(exp)).round() as u32;
    match mantissa {
        1 | 5 => 5, // odd
        2 => 4,     // even
        _ => 5,
    }
}
