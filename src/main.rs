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
    commands.spawn((Camera2d, GraphZoom::default()));
}

fn handle_zoom_input(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    mut graph_zoom: Single<&mut GraphZoom, With<Camera2d>>,
) {
    for event in mouse_wheel_events.read() {
        graph_zoom.zoom -= event.y * 0.6;
    }
    graph_zoom.zoom = graph_zoom.zoom.clamp(0.1, 10.0);
}

fn handle_pan_input(
    mut mouse_motion_messages: MessageReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_transform: Single<&mut Transform, With<Camera2d>>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }

    // 2. Loop through the mouse motion entries
    for motion in mouse_motion_messages.read() {
        // motion.delta is a Vec2 containing how many pixels the mouse moved this frame.
        // Remember: Moving the camera LEFT makes the world look like it's moving RIGHT!
        camera_transform.translation.x -= motion.delta.x;
        camera_transform.translation.y += motion.delta.y;
    }
}

/// Renders a coordinate grid that tracks the camera position endlessly
fn draw_infinite_grid(
    mut gizmos: Gizmos,
    window: Single<&Window>,
    camera_transform: Single<&Transform, With<Camera2d>>,
    camera_zoom: Single<&GraphZoom, With<Camera2d>>,
) {
    // 1. Calculate the visible world bounds currently inside the viewport
    let viewport_width = window.width();
    let viewport_height = window.height();

    // 2. Define the graph cell spacing size in pixels/units
    let cell_size = DEFAULT_CELL_SIZE / camera_zoom.zoom;

    // 3. Calculate how many lines fit on screen, forcing an even count
    // to maintain a perfectly centered line layout.
    let half_cols: u32 = (((viewport_width / 2.0) / cell_size).ceil() as u32).max(1);
    let half_rows = (((viewport_height / 2.0) / cell_size).ceil() as u32).max(1);

    let cols = half_cols * 2;
    let rows = half_rows * 2;

    // 4. Snapping trick: anchor grid position to camera but snap to nearest cell step
    let cam_x = camera_transform.translation.x;
    let cam_y = camera_transform.translation.y;
    let snapped_x = (cam_x / cell_size).round() * cell_size;
    let snapped_y = (cam_y / cell_size).round() * cell_size;

    // Render the infinite background grid
    gizmos.grid_2d(
        Isometry2d::from_translation(Vec2::new(snapped_x, snapped_y)),
        UVec2::new(cols, rows),
        Vec2::new(cell_size, cell_size),
        LinearRgba::gray(0.15), // Subtle grid lines
    );

    // Draw the Bold Primary Graph Axes (X = 0 and Y = 0)
    let half_w = viewport_width;
    let half_h = viewport_height;

    gizmos.line_2d(
        Vec2::new(cam_x - half_w, 0.0),
        Vec2::new(cam_x + half_w, 0.0),
        Color::WHITE,
    ); // X-Axis

    gizmos.line_2d(
        Vec2::new(0.0, cam_y - half_h),
        Vec2::new(0.0, cam_y + half_h),
        Color::WHITE,
    ); // Y-Axis
}

#[derive(Component, PartialEq, PartialOrd)]
struct GraphZoom {
    zoom: f32,
}

impl Default for GraphZoom {
    fn default() -> Self {
        Self { zoom: 1.0 }
    }
}
