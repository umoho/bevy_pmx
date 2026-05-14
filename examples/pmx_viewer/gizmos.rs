use bevy::pbr::wireframe::{Wireframe, WireframeColor};
use bevy::{prelude::*, transform::TransformSystems};

use crate::scene::{LoadedScene, ViewerPrimitive};

const BONE_TOGGLE_KEY: KeyCode = KeyCode::KeyB;
const PRIMITIVE_TOGGLE_KEY: KeyCode = KeyCode::KeyP;
const BONE_LINK_COLOR: Color = Color::srgba(0.35, 0.85, 1.0, 0.95);
const BONE_NODE_COLOR: Color = Color::srgba(0.97, 0.78, 0.25, 0.98);
const ROOT_BONE_COLOR: Color = Color::srgba(0.95, 0.55, 0.25, 0.98);
const PRIMITIVE_WIREFRAME_COLOR: Color = Color::srgba(0.96, 0.92, 0.45, 0.97);
const HUD_BACKGROUND: Color = Color::srgba(0.08, 0.09, 0.12, 0.65);
const HUD_TEXT_COLOR: Color = Color::srgba(0.96, 0.97, 0.98, 0.98);

#[derive(Resource, Debug, Clone)]
pub(crate) struct ViewerGizmoSettings {
    pub(crate) show_bones: bool,
    pub(crate) show_primitive_wireframes: bool,
}

impl Default for ViewerGizmoSettings {
    fn default() -> Self {
        Self {
            show_bones: false,
            show_primitive_wireframes: false,
        }
    }
}

#[derive(Component)]
struct ViewerHud;

pub(crate) struct ViewerGizmoPlugin;

impl Plugin for ViewerGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewerGizmoSettings>()
            .add_systems(Update, (toggle_gizmos_on_hotkey, update_gizmo_hud).chain())
            .add_systems(
                PostUpdate,
                sync_primitive_wireframes.after(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                draw_bone_gizmos.after(TransformSystems::Propagate),
            );
    }
}

pub(crate) fn spawn_gizmo_overlay(commands: &mut Commands, settings: &ViewerGizmoSettings) {
    let ui_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ))
        .id();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12.0),
                left: px(12.0),
                padding: UiRect::all(px(8.0)),
                ..default()
            },
            BackgroundColor(HUD_BACKGROUND),
            UiTargetCamera(ui_camera),
        ))
        .with_child((
            ViewerHud,
            Text::new(hud_text(settings)),
            TextFont::from_font_size(18.0),
            TextColor(HUD_TEXT_COLOR),
        ));
}

fn toggle_gizmos_on_hotkey(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ViewerGizmoSettings>,
) {
    if keyboard.just_pressed(BONE_TOGGLE_KEY) {
        settings.show_bones = !settings.show_bones;
    }

    if keyboard.just_pressed(PRIMITIVE_TOGGLE_KEY) {
        settings.show_primitive_wireframes = !settings.show_primitive_wireframes;
    }
}

fn update_gizmo_hud(
    settings: Res<ViewerGizmoSettings>,
    mut text: Query<&mut Text, With<ViewerHud>>,
) {
    if !settings.is_changed() {
        return;
    }

    let Ok(mut text) = text.single_mut() else {
        return;
    };

    **text = hud_text(&settings);
}

fn sync_primitive_wireframes(
    settings: Res<ViewerGizmoSettings>,
    primitives: Query<(Entity, Option<&Wireframe>, Option<&WireframeColor>), With<ViewerPrimitive>>,
    mut commands: Commands,
) {
    let should_show = settings.show_primitive_wireframes;

    for (entity, maybe_wireframe, maybe_color) in &primitives {
        if should_show {
            if maybe_wireframe.is_none() || maybe_color.is_none() {
                commands.entity(entity).insert((
                    Wireframe,
                    WireframeColor {
                        color: PRIMITIVE_WIREFRAME_COLOR,
                    },
                ));
            }
        } else if maybe_wireframe.is_some() || maybe_color.is_some() {
            commands.entity(entity).remove::<Wireframe>();
            commands.entity(entity).remove::<WireframeColor>();
        }
    }
}

fn draw_bone_gizmos(
    settings: Res<ViewerGizmoSettings>,
    scene: Res<LoadedScene>,
    mut gizmos: Gizmos,
) {
    if !settings.show_bones {
        return;
    }

    let model_offset = -scene.bounds_center;
    let node_radius = (scene.bounds_radius * 0.015).clamp(0.015, 0.07);
    let root_radius = node_radius * 1.25;
    let bones = scene.model.bone_records();

    for bone in bones {
        let bone_position = Vec3::from(bone.position) + model_offset;
        let is_root = bone.parent_index.is_none();
        let node_color = if is_root {
            ROOT_BONE_COLOR
        } else {
            BONE_NODE_COLOR
        };

        if let Some(parent_index) = bone.parent_index {
            let parent_position = Vec3::from(bones[parent_index].position) + model_offset;
            gizmos.line(parent_position, bone_position, BONE_LINK_COLOR);
        }

        gizmos.sphere(
            Isometry3d::from_translation(bone_position),
            if is_root { root_radius } else { node_radius },
            node_color,
        );
    }
}

fn hud_text(settings: &ViewerGizmoSettings) -> String {
    format!(
        "B bones: {}\nP wireframe: {}",
        status_word(settings.show_bones),
        status_word(settings.show_primitive_wireframes)
    )
}

fn status_word(enabled: bool) -> &'static str {
    if enabled { "show" } else { "hidden" }
}
