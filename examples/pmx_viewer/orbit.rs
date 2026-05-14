use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::Projection,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    transform::TransformSystems,
};

use crate::scene::LoadedScene;

const BASE_MIN_DISTANCE: f32 = 2.0;
const BASE_MAX_DISTANCE: f32 = 150.0;
const DEFAULT_YAW: f32 = -0.42;
const DEFAULT_PITCH: f32 = -0.2;
const TARGET_HEIGHT_RATIO: f32 = 0.68;
const DISTANCE_PADDING: f32 = 1.15;

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct OrbitCamera {
    pub(crate) target: Vec3,
    pub(crate) distance: f32,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) orbit_velocity: Vec2,
    pub(crate) zoom_velocity: f32,
    pub(crate) pan_velocity: Vec2,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 1.0,
            yaw: DEFAULT_YAW,
            pitch: DEFAULT_PITCH,
            orbit_velocity: Vec2::ZERO,
            zoom_velocity: 0.0,
            pan_velocity: Vec2::ZERO,
        }
    }
}

impl OrbitCamera {
    pub(crate) fn apply_to_transform(&self, transform: &mut Transform) {
        let rotation = Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        transform.translation = self.target + rotation * Vec3::new(0.0, 0.0, self.distance);
        transform.look_at(self.target, Vec3::Y);
    }
}

#[derive(Resource)]
struct OrbitCameraSettings {
    orbit_sensitivity: f32,
    zoom_sensitivity: f32,
    pan_sensitivity: f32,
    damping: f32,
    pan_damping: f32,
}

#[derive(Resource, Default)]
struct InitialOrbitFramingState {
    framed_scene: bool,
}

impl Default for OrbitCameraSettings {
    fn default() -> Self {
        Self {
            orbit_sensitivity: 0.005,
            zoom_sensitivity: 0.025,
            pan_sensitivity: 0.0010,
            damping: 14.0,
            pan_damping: 20.0,
        }
    }
}

pub(crate) struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OrbitCameraSettings>()
            .init_resource::<InitialOrbitFramingState>()
            .add_systems(Update, orbit_camera)
            .add_systems(
                PostUpdate,
                frame_initial_orbit.after(TransformSystems::Propagate),
            );
    }
}

pub(crate) fn spawn_orbit_camera(commands: &mut Commands, scene: &LoadedScene) {
    let mut orbit = OrbitCamera::default();
    let mut transform = Transform::default();
    let (world_min, world_max) = scene.world_bounds();
    frame_orbit(
        scene,
        &mut orbit,
        &mut transform,
        world_min,
        world_max,
        std::f32::consts::FRAC_PI_3,
        1.0,
    );

    commands.spawn((
        Name::new("Orbit Camera"),
        Camera3d::default(),
        transform,
        orbit,
    ));
}

fn orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    settings: Res<OrbitCameraSettings>,
    scene: Res<LoadedScene>,
    mut cameras: Query<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
) {
    const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.05;
    let (min_distance, max_distance) = distance_limits(scene.bounds_radius);

    for (mut transform, mut orbit) in &mut cameras {
        if mouse_buttons.pressed(MouseButton::Left) {
            let orbit_delta = Vec2::new(-mouse_motion.delta.x, -mouse_motion.delta.y)
                * settings.orbit_sensitivity;
            orbit.yaw += orbit_delta.x;
            orbit.pitch += orbit_delta.y;
            orbit.orbit_velocity = orbit_delta * 60.0;
        }

        if mouse_scroll.delta != Vec2::ZERO {
            let is_pan = keyboard.any_pressed([KeyCode::Space]);
            let is_zoom = keyboard.any_pressed([
                KeyCode::ControlLeft,
                KeyCode::ControlRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ]);
            if is_pan {
                let distance = orbit.distance;
                orbit.pan_velocity += Vec2::new(-mouse_scroll.delta.x, mouse_scroll.delta.y)
                    * settings.pan_sensitivity
                    * distance
                    * 30.0;
            } else if is_zoom {
                orbit.zoom_velocity += mouse_scroll.delta.y * settings.zoom_sensitivity * 30.0;
            } else {
                orbit.orbit_velocity += Vec2::new(-mouse_scroll.delta.x, -mouse_scroll.delta.y)
                    * settings.orbit_sensitivity
                    * 20.0;
            }
        }

        let dt = time.delta_secs();
        orbit.yaw += orbit.orbit_velocity.x * dt;
        orbit.pitch = (orbit.pitch + orbit.orbit_velocity.y * dt).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        orbit.distance =
            (orbit.distance + orbit.zoom_velocity * dt).clamp(min_distance, max_distance);

        if orbit.pan_velocity != Vec2::ZERO {
            let pan_velocity = orbit.pan_velocity;
            let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
            let right = rotation * Vec3::X;
            let up = rotation * Vec3::Y;
            orbit.target += (right * pan_velocity.x + up * pan_velocity.y) * dt;
        }

        let drag = (-settings.damping * dt).exp();
        orbit.orbit_velocity *= drag;
        orbit.zoom_velocity *= drag;
        let pan_drag = (-settings.pan_damping * dt).exp();
        orbit.pan_velocity *= pan_drag;

        orbit.apply_to_transform(&mut transform);
    }
}

fn frame_initial_orbit(
    mut state: ResMut<InitialOrbitFramingState>,
    scene: Res<LoadedScene>,
    mut cameras: Query<(&mut Transform, &mut OrbitCamera, &Projection), With<Camera3d>>,
) {
    if state.framed_scene {
        return;
    }

    let Some((mut transform, mut orbit, projection)) = cameras.iter_mut().next() else {
        return;
    };
    let Projection::Perspective(perspective) = projection else {
        return;
    };

    let (world_min, world_max) = scene.world_bounds();
    frame_orbit(
        &scene,
        &mut orbit,
        &mut transform,
        world_min,
        world_max,
        perspective.fov,
        perspective.aspect_ratio,
    );

    state.framed_scene = true;
    info!(
        "Initial orbit framed for {} target={:?} distance={}",
        scene.path.display(),
        orbit.target,
        orbit.distance
    );
}

fn frame_orbit(
    scene: &LoadedScene,
    orbit: &mut OrbitCamera,
    transform: &mut Transform,
    world_min: Vec3,
    world_max: Vec3,
    vertical_fov: f32,
    aspect_ratio: f32,
) {
    let target = biased_target_from_bounds(world_min, world_max);
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let (min_distance, max_distance) = distance_limits(scene.bounds_radius);
    let distance = estimate_orbit_distance(
        world_min,
        world_max,
        target,
        rotation,
        vertical_fov,
        aspect_ratio,
    )
    .clamp(min_distance, max_distance);

    orbit.target = target;
    orbit.distance = distance;
    orbit.orbit_velocity = Vec2::ZERO;
    orbit.zoom_velocity = 0.0;
    orbit.pan_velocity = Vec2::ZERO;
    orbit.apply_to_transform(transform);
}

fn biased_target_from_bounds(min: Vec3, max: Vec3) -> Vec3 {
    let center = (min + max) * 0.5;
    let target_y = min.y + (max.y - min.y) * TARGET_HEIGHT_RATIO;
    Vec3::new(center.x, target_y, center.z)
}

fn estimate_orbit_distance(
    aabb_min: Vec3,
    aabb_max: Vec3,
    target: Vec3,
    rotation: Quat,
    vertical_fov: f32,
    aspect_ratio: f32,
) -> f32 {
    let tan_vertical = (vertical_fov * 0.5).tan();
    if !tan_vertical.is_finite() || tan_vertical <= 0.0 {
        return BASE_MIN_DISTANCE;
    }

    let tan_horizontal = tan_vertical * aspect_ratio.max(0.01);
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let forward = -(rotation * Vec3::Z);
    let mut required_distance: f32 = 0.0;

    for corner in aabb_corners(aabb_min, aabb_max) {
        let offset = corner - target;
        let depth = offset.dot(forward);
        let horizontal = offset.dot(right).abs();
        let vertical = offset.dot(up).abs();

        required_distance = required_distance.max(-depth);
        required_distance = required_distance.max(horizontal / tan_horizontal - depth);
        required_distance = required_distance.max(vertical / tan_vertical - depth);
    }

    (required_distance.max(0.0_f32) * DISTANCE_PADDING).max(BASE_MIN_DISTANCE)
}

fn aabb_corners(aabb_min: Vec3, aabb_max: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(aabb_min.x, aabb_min.y, aabb_min.z),
        Vec3::new(aabb_max.x, aabb_min.y, aabb_min.z),
        Vec3::new(aabb_min.x, aabb_max.y, aabb_min.z),
        Vec3::new(aabb_max.x, aabb_max.y, aabb_min.z),
        Vec3::new(aabb_min.x, aabb_min.y, aabb_max.z),
        Vec3::new(aabb_max.x, aabb_min.y, aabb_max.z),
        Vec3::new(aabb_min.x, aabb_max.y, aabb_max.z),
        Vec3::new(aabb_max.x, aabb_max.y, aabb_max.z),
    ]
}

fn distance_limits(bounds_radius: f32) -> (f32, f32) {
    let scale = bounds_radius.max(0.01);
    (BASE_MIN_DISTANCE * scale, BASE_MAX_DISTANCE * scale)
}
