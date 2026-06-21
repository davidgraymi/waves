use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GraphPlugin)
        .run();
}

const DEFAULT_CELL_SIZE: f32 = 40.0;
const SCALE_DAMPING: f32 = 0.005;

struct GraphPlugin;

impl Plugin for GraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
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

    // Cursor offset from viewport center in world-space direction (flip y: screen y-down → world y-up)
    let viewport_center = Vec2::new(window.width(), window.height()) / 2.0;
    let cursor_from_center = Vec2::new(
        cursor_screen.x - viewport_center.x,
        -(cursor_screen.y - viewport_center.y),
    );

    for event in mouse_wheel_events.read() {
        let old_scale = camera_transform.scale.x;
        // Scroll up (event.y > 0) → zoom in → smaller scale (camera sees fewer world units)
        let new_scale = (old_scale / (1.0 + event.y * SCALE_DAMPING)).clamp(0.05, 10.0);

        // Keep the world point under the cursor fixed:
        // world_cursor = camera_pos + cursor_from_center * old_scale
        // new_camera_pos = world_cursor - cursor_from_center * new_scale
        //                = camera_pos + cursor_from_center * (old_scale - new_scale)
        let scale_delta = old_scale - new_scale;
        camera_transform.translation.x += cursor_from_center.x * scale_delta;
        camera_transform.translation.y += cursor_from_center.y * scale_delta;
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

    // World units visible through the viewport at the current zoom level
    let world_width = viewport_width * camera_scale;
    let world_height = viewport_height * camera_scale;

    let cell_size = DEFAULT_CELL_SIZE;

    let half_cols = ((world_width / 2.0) / cell_size).ceil() as u32 + 1;
    let half_rows = ((world_height / 2.0) / cell_size).ceil() as u32 + 1;
    let cols = (half_cols * 2).max(2);
    let rows = (half_rows * 2).max(2);

    let cam_x = camera_transform.translation.x;
    let cam_y = camera_transform.translation.y;
    let snapped_x = (cam_x / cell_size).round() * cell_size;
    let snapped_y = (cam_y / cell_size).round() * cell_size;

    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::new(snapped_x, snapped_y)),
        UVec2::new(cols, rows),
        Vec2::new(cell_size, cell_size),
        LinearRgba::gray(0.15),
    );

    // Bold primary axes spanning the visible world area
    gizmos.line_2d(
        Vec2::new(cam_x - world_width, 0.0),
        Vec2::new(cam_x + world_width, 0.0),
        Color::WHITE,
    );
    gizmos.line_2d(
        Vec2::new(0.0, cam_y - world_height),
        Vec2::new(0.0, cam_y + world_height),
        Color::WHITE,
    );
}
