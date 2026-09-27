//! Select and dispatch the best contextual interaction near the surface player.

use bevy::{input::mouse::MouseButton, prelude::*};

use crate::player_movement::SurfacePlayer;

/// Maximum angle from the player's tangent forward direction for a target to qualify.
pub const INTERACTION_FORWARD_DOT: f32 = 0.5;
/// Default reach for interactables, in world units.
pub const DEFAULT_INTERACTION_RANGE: f32 = 3.0;

/// An entity that can be selected by the contextual interaction system.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Interactable {
    /// Action text shown after the `[E]` prompt prefix.
    pub prompt: String,
    /// Maximum world-space distance at which this target can be selected.
    pub range: f32,
    /// Approximate world-space radius used by cursor ray picking.
    pub pick_radius: f32,
}

impl Interactable {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            range: DEFAULT_INTERACTION_RANGE,
            pick_radius: 1.0,
        }
    }

    pub fn with_pick_radius(mut self, radius: f32) -> Self {
        self.pick_radius = radius;
        self
    }
}

/// The sole selected interaction and the prompt text currently presented to the player.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectedInteraction(pub Option<InteractionPromptData>);

/// The interactable currently under the cursor, independent of contextual range selection.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoveredInteractable(pub Option<Entity>);

#[derive(Component)]
struct HoverScale(Vec3);

/// Stable presentation and dispatch data for the selected target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionPromptData {
    pub entity: Entity,
    pub text: String,
}

/// Emitted once when E or left-click is pressed while an interaction target is selected.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionRequested {
    pub target: Entity,
}

#[derive(Component)]
struct InteractionPromptText;

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedInteraction>()
            .init_resource::<HoveredInteractable>()
            .add_message::<InteractionRequested>()
            .add_systems(Startup, spawn_prompt)
            .add_systems(
                Update,
                (
                    select_interaction,
                    select_mouse_hover,
                    update_hover_highlight,
                    dispatch_interaction,
                    update_prompt_text,
                )
                    .chain()
                    .after(crate::player_movement::move_surface_players),
            );
    }
}

fn spawn_prompt(mut commands: Commands) {
    commands.spawn((
        InteractionPromptText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            left: px(16),
            ..default()
        },
    ));
}

fn select_mouse_hover(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    candidates: Query<(Entity, &Interactable, &GlobalTransform)>,
    mut hovered: ResMut<HoveredInteractable>,
) {
    let next = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .and_then(|cursor| {
            let (camera, transform) = cameras.single().ok()?;
            let ray = camera.viewport_to_world(transform, cursor).ok()?;
            choose_ray_interactable(
                ray.origin,
                *ray.direction,
                candidates.iter().map(|(entity, interactable, transform)| {
                    (entity, transform.translation(), interactable.pick_radius)
                }),
            )
        });
    if hovered.0 != next {
        hovered.0 = next;
    }
}

/// Pick the nearest interactable sphere intersected by a camera ray.
pub fn choose_ray_interactable(
    origin: Vec3,
    direction: Vec3,
    candidates: impl IntoIterator<Item = (Entity, Vec3, f32)>,
) -> Option<Entity> {
    let direction = direction.try_normalize()?;
    candidates
        .into_iter()
        .filter_map(|(entity, center, radius)| {
            if !center.is_finite() || !radius.is_finite() || radius <= 0.0 {
                return None;
            }
            let offset = origin - center;
            let b = offset.dot(direction);
            let c = offset.length_squared() - radius * radius;
            let discriminant = b * b - c;
            if discriminant < 0.0 || !discriminant.is_finite() {
                return None;
            }
            let root = discriminant.sqrt();
            let distance = [-b - root, -b + root]
                .into_iter()
                .filter(|distance| *distance >= 0.0)
                .min_by(f32::total_cmp)?;
            Some((distance, entity.to_bits(), entity))
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        })
        .map(|(_, _, entity)| entity)
}

fn update_hover_highlight(
    mut commands: Commands,
    hovered: Res<HoveredInteractable>,
    mut candidates: Query<(Entity, &mut Transform, Option<&HoverScale>), With<Interactable>>,
) {
    if !hovered.is_changed() {
        return;
    }
    for (entity, mut transform, scale) in &mut candidates {
        if hovered.0 == Some(entity) {
            if scale.is_none() {
                let original = transform.scale;
                transform.scale = original * 1.12;
                commands.entity(entity).insert(HoverScale(original));
            }
        } else if let Some(original) = scale {
            transform.scale = original.0;
            commands.entity(entity).remove::<HoverScale>();
        }
    }
}

fn select_interaction(
    players: Query<(&SurfacePlayer, &Transform)>,
    candidates: Query<(Entity, &Interactable, &Transform)>,
    mut selected: ResMut<SelectedInteraction>,
) {
    let Ok((player, player_transform)) = players.single() else {
        selected.0 = None;
        return;
    };
    let up = player.location.direction.normalize_or_zero();
    let forward = (player.heading - up * player.heading.dot(up)).normalize_or_zero();
    selected.0 = choose_interaction(
        player_transform.translation,
        up,
        forward,
        candidates
            .iter()
            .map(|(entity, interactable, transform)| InteractionCandidate {
                entity,
                position: transform.translation,
                prompt: &interactable.prompt,
                range: interactable.range,
            }),
    );
}

struct InteractionCandidate<'a> {
    entity: Entity,
    position: Vec3,
    prompt: &'a str,
    range: f32,
}

/// Eligible candidates must be within their own range and within 60° of forward;
/// among those, the nearest wins, with entity ID as a deterministic exact-tie break.
fn choose_interaction<'a>(
    player_position: Vec3,
    up: Vec3,
    forward: Vec3,
    candidates: impl IntoIterator<Item = InteractionCandidate<'a>>,
) -> Option<InteractionPromptData> {
    let up = up.try_normalize()?;
    let forward = (forward - up * forward.dot(up)).try_normalize()?;
    candidates
        .into_iter()
        .filter_map(|candidate| {
            if !candidate.position.is_finite()
                || !candidate.range.is_finite()
                || candidate.range < 0.0
            {
                return None;
            }
            let offset = candidate.position - player_position;
            let distance_squared = offset.length_squared();
            if !distance_squared.is_finite() || distance_squared > candidate.range * candidate.range
            {
                return None;
            }
            let tangent_offset = offset - up * offset.dot(up);
            let direction = tangent_offset.try_normalize()?;
            if direction.dot(forward) < INTERACTION_FORWARD_DOT {
                return None;
            }
            Some((distance_squared, candidate.entity.to_bits(), candidate))
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        })
        .map(|(_, _, candidate)| InteractionPromptData {
            entity: candidate.entity,
            text: format!("[E] {}", candidate.prompt),
        })
}

pub(crate) fn dispatch_interaction(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    placement: Option<Res<crate::placement::PlacementMode>>,
    selected: Res<SelectedInteraction>,
    mut requests: MessageWriter<InteractionRequested>,
) {
    if placement.is_some_and(|mode| mode.active) {
        return;
    }
    if (keyboard.just_pressed(KeyCode::KeyE) || mouse.just_pressed(MouseButton::Left))
        && let Some(selected) = &selected.0
    {
        requests.write(InteractionRequested {
            target: selected.entity,
        });
    }
}

fn update_prompt_text(
    selected: Res<SelectedInteraction>,
    mut labels: Query<&mut Text, With<InteractionPromptText>>,
) {
    if !selected.is_changed() {
        return;
    }
    let text = selected
        .0
        .as_ref()
        .map_or_else(String::new, |prompt| prompt.text.clone());
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        entity: u32,
        position: Vec3,
        prompt: &'static str,
        range: f32,
    ) -> InteractionCandidate<'static> {
        InteractionCandidate {
            entity: Entity::from_raw_u32(entity).expect("test entity index is valid"),
            position,
            prompt,
            range,
        }
    }

    #[test]
    fn selects_only_in_range_targets() {
        let selected = choose_interaction(
            Vec3::ZERO,
            Vec3::Y,
            Vec3::Z,
            [
                candidate(1, Vec3::Z * 2.0, "Gather clay", 2.0),
                candidate(2, Vec3::Z * 2.1, "Use kiln", 2.0),
            ],
        );
        assert_eq!(
            selected.unwrap().entity,
            Entity::from_raw_u32(1).expect("test entity index is valid")
        );
    }

    #[test]
    fn rejects_targets_outside_the_forward_cone() {
        let selected = choose_interaction(
            Vec3::ZERO,
            Vec3::Y,
            Vec3::Z,
            [candidate(1, Vec3::X * 2.0, "Talk", 3.0)],
        );
        assert!(selected.is_none());
    }

    #[test]
    fn nearest_valid_target_wins_and_prompt_matches_it() {
        let selected = choose_interaction(
            Vec3::ZERO,
            Vec3::Y,
            Vec3::Z,
            [
                candidate(1, Vec3::Z * 2.5, "Far", 3.0),
                candidate(2, Vec3::Z * 1.5, "Gather clay", 3.0),
            ],
        )
        .unwrap();
        assert_eq!(
            selected.entity,
            Entity::from_raw_u32(2).expect("test entity index is valid")
        );
        assert_eq!(selected.text, "[E] Gather clay");
    }

    #[test]
    fn exact_distance_ties_use_entity_id_and_no_candidates_clear_selection() {
        let tied = choose_interaction(
            Vec3::ZERO,
            Vec3::Y,
            Vec3::Z,
            [
                candidate(2, Vec3::Z * 1.0, "Second", 3.0),
                candidate(1, Vec3::Z * 1.0, "First", 3.0),
            ],
        )
        .unwrap();
        let expected = [
            Entity::from_raw_u32(1).expect("test entity index is valid"),
            Entity::from_raw_u32(2).expect("test entity index is valid"),
        ]
        .into_iter()
        .min_by_key(|entity| entity.to_bits())
        .unwrap();
        assert_eq!(tied.entity, expected);
        assert!(choose_interaction(Vec3::ZERO, Vec3::Y, Vec3::Z, []).is_none());
    }

    #[test]
    fn camera_ray_picks_nearest_interactable_and_ignores_misses() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        assert_eq!(
            choose_ray_interactable(
                Vec3::ZERO,
                Vec3::Z,
                [(far, Vec3::Z * 5.0, 1.0), (near, Vec3::Z * 3.0, 0.5)],
            ),
            Some(near)
        );
        assert_eq!(
            choose_ray_interactable(Vec3::ZERO, Vec3::X, [(near, Vec3::Z, 0.5)]),
            None
        );
    }

    #[test]
    fn hover_highlight_scales_the_target_and_restores_it_when_cleared() {
        let mut app = App::new();
        app.init_resource::<HoveredInteractable>()
            .add_systems(Update, update_hover_highlight);
        let entity = app
            .world_mut()
            .spawn((
                Interactable::new("Test"),
                Transform::from_scale(Vec3::splat(2.0)),
            ))
            .id();
        app.world_mut().resource_mut::<HoveredInteractable>().0 = Some(entity);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            Vec3::splat(2.24)
        );
        app.world_mut().resource_mut::<HoveredInteractable>().0 = None;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            Vec3::splat(2.0)
        );
    }

    #[test]
    fn left_click_dispatches_the_same_selected_target_as_e() {
        let mut app = App::new();
        app.add_message::<InteractionRequested>()
            .init_resource::<SelectedInteraction>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, dispatch_interaction);
        let target = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<SelectedInteraction>().0 = Some(InteractionPromptData {
            entity: target,
            text: "[E] Test interaction".to_owned(),
        });
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        let messages = app.world().resource::<Messages<InteractionRequested>>();
        let sent: Vec<_> = messages.iter_current_update_messages().collect();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].target, target);
    }
}
