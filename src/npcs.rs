//! Primitive NPC characters and their named, surface-aligned properties.

use bevy::{input::mouse::MouseButton, prelude::*};

use crate::{
    ceramics::{CeramicForm, CeramicItem, ProcessingState},
    economy::Merchant,
    game_clock::GameClock,
    interaction::{Interactable, InteractionRequested},
    inventory::{CeramicObjectId, Inventory},
    planet::{
        DEFAULT_PLANET_RADIUS, FaceOrientation, PlanetCoordinate, PlanetFace, PlanetTile,
        TileCoordinate, sample_tile_surface,
    },
    surface_transform::{SurfaceLocation, surface_transform},
    workbench::CraftedCeramics,
};

/// The three named people who inhabit the starter world.
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum NpcCharacter {
    Baker,
    Carpenter,
    Merchant,
}

/// A named NPC destination/property in the world.
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum NpcProperty {
    Bakery,
    Workshop,
    GeneralStore,
}

/// Friendship is bounded to the prototype's single 0–100 relationship value.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NpcFriendship(pub u8);

impl NpcFriendship {
    pub fn increase(&mut self, amount: u8) {
        self.0 = self.0.saturating_add(amount).min(100);
    }
}

/// Authored category used to select an NPC's contextual dialogue pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcDialogueKind {
    Generic,
    Request,
    Relationship,
}

/// The currently displayed authored NPC line, if any.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct NpcDialogueDisplay(pub Option<NpcDialogueLine>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcDialogueLine {
    pub speaker: NpcCharacter,
    pub kind: NpcDialogueKind,
    pub text: &'static str,
}

/// A request from a gameplay/UI caller to gift one owned ceramic to an NPC.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcGiftRequested {
    pub npc: Entity,
    pub item: CeramicObjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiftResult {
    Liked,
    NotLiked,
    NotOwned,
    UnknownCeramic,
    NotFired,
}

/// A deterministic phase in an NPC's authored daily routine.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcScheduleState {
    Home,
    Travel,
    Work,
    Social,
}

/// Named authored locations used by NPC schedules and exposed for debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcDestination {
    Home,
    Work,
    Social,
}

/// Inspectable schedule snapshot attached to each NPC entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcScheduleDebug {
    pub state: NpcScheduleState,
    pub destination: NpcDestination,
    /// Index into the active authored route (zero while stationary).
    pub waypoint_index: usize,
}

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcDialogueDisplay>()
            .init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .add_message::<InteractionRequested>()
            .add_message::<NpcGiftRequested>()
            .add_systems(Startup, (spawn_npc_world, spawn_dialogue_ui))
            .add_systems(
                Update,
                (
                    update_npc_schedules,
                    handle_npc_talks,
                    handle_npc_gifts,
                    cancel_npc_dialogue,
                    update_dialogue_ui,
                )
                    .chain(),
            );
    }
}

struct NpcDefinition {
    character: NpcCharacter,
    character_name: &'static str,
    property: NpcProperty,
    property_name: &'static str,
    interaction_prompt: &'static str,
    character_tile: (u8, u8),
    property_tile: (u8, u8),
    body_color: Color,
    accessory_color: Color,
}

const NPCS: [NpcDefinition; 3] = [
    NpcDefinition {
        character: NpcCharacter::Baker,
        character_name: "Baker",
        property: NpcProperty::Bakery,
        property_name: "Bakery",
        interaction_prompt: "Talk to the Baker",
        character_tile: (8, 12),
        property_tile: (9, 12),
        body_color: Color::srgb(0.92, 0.89, 0.81),
        accessory_color: Color::srgb(0.96, 0.96, 0.91),
    },
    NpcDefinition {
        character: NpcCharacter::Carpenter,
        character_name: "Carpenter",
        property: NpcProperty::Workshop,
        property_name: "Workshop",
        interaction_prompt: "Talk to the Carpenter",
        character_tile: (11, 12),
        property_tile: (12, 12),
        body_color: Color::srgb(0.43, 0.25, 0.14),
        accessory_color: Color::srgb(0.29, 0.17, 0.09),
    },
    NpcDefinition {
        character: NpcCharacter::Merchant,
        character_name: "Merchant",
        property: NpcProperty::GeneralStore,
        property_name: "General Store",
        interaction_prompt: "Trade with the Merchant",
        character_tile: (14, 12),
        property_tile: (15, 12),
        body_color: Color::srgb(0.13, 0.36, 0.76),
        accessory_color: Color::srgb(0.30, 0.58, 0.91),
    },
];

const SOCIAL_TILE: (u8, u8) = (12, 16);

#[derive(Debug, Clone, Copy)]
pub struct NpcDialoguePools {
    pub generic: &'static [&'static str],
    pub request: &'static [&'static str],
    pub relationship: &'static [&'static str],
}

const BAKER_DIALOGUE: NpcDialoguePools = NpcDialoguePools {
    generic: &[
        "The morning bread smells best when it is still warm.",
        "I keep running out of cups. People apparently like drinking things.",
        "A quiet bakery is a good place to plan tomorrow's batch.",
        "The meadow clay makes a lovely little cup.",
        "Come by later; the ovens are busy right now.",
    ],
    request: &[
        "Could you make a few cups for the bakery counter?",
        "A pair of sturdy bowls would be perfect for serving soup.",
        "Your pottery would make this little bakery feel like home.",
    ],
    relationship: &[
        "You have become one of my favorite regulars.",
        "I saved you the first warm loaf. That's what friends are for.",
        "The bakery feels brighter whenever you stop by.",
    ],
};

const CARPENTER_DIALOGUE: NpcDialoguePools = NpcDialoguePools {
    generic: &[
        "Measure twice, cut once. Then measure once more for luck.",
        "A good shelf should make the things on it feel proud.",
        "The workshop is quietest before the first saw starts.",
        "I can fix most things with patience and a steady hand.",
        "That ceramic would look right at home on a timber shelf.",
    ],
    request: &[
        "Could you make a bowl for the workbench? It catches every loose nail.",
        "A vase would brighten up this pile of lumber.",
        "Bring me a fired piece and I'll find it a place in the workshop.",
    ],
    relationship: &[
        "I built you a little hook for your workshop apron.",
        "I trust your eye for a good shape. That means something to me.",
        "There's always room at my workbench for a friend.",
    ],
};

const MERCHANT_DIALOGUE: NpcDialoguePools = NpcDialoguePools {
    generic: &[
        "A good shopkeeper knows when to listen as well as when to bargain.",
        "Every little object has a story. Some just need the right shelf.",
        "The coast has been bringing in the loveliest pale clay lately.",
        "Take your time browsing; there's no rush in this little town.",
        "I keep a ledger, but I never write down a kind favor.",
    ],
    request: &[
        "Could you bring a fired cup for the front window display?",
        "A bowl with a bright glaze would catch every customer's eye.",
        "I would love to stock one of your vases in the shop.",
    ],
    relationship: &[
        "For you, my friend, I'll always save the best spot in the window.",
        "You have a knack for finding treasures worth keeping.",
        "No ledger could measure how much I appreciate your visits.",
    ],
};

/// Authored preferences. Gifts outside these lists do not increase friendship.
pub const NPC_LIKED_CERAMIC_FORMS: [(NpcCharacter, &[CeramicForm]); 3] = [
    (NpcCharacter::Baker, &[CeramicForm::Cup, CeramicForm::Bowl]),
    (
        NpcCharacter::Carpenter,
        &[CeramicForm::Bowl, CeramicForm::Vase],
    ),
    (
        NpcCharacter::Merchant,
        &[CeramicForm::Cup, CeramicForm::Bowl, CeramicForm::Vase],
    ),
];

pub fn dialogue_pools(character: NpcCharacter) -> &'static NpcDialoguePools {
    match character {
        NpcCharacter::Baker => &BAKER_DIALOGUE,
        NpcCharacter::Carpenter => &CARPENTER_DIALOGUE,
        NpcCharacter::Merchant => &MERCHANT_DIALOGUE,
    }
}

/// Select an authored line by stable ordinal; no text is generated at runtime.
pub fn select_dialogue(
    character: NpcCharacter,
    kind: NpcDialogueKind,
    ordinal: usize,
) -> &'static str {
    let pools = dialogue_pools(character);
    let lines = match kind {
        NpcDialogueKind::Generic => pools.generic,
        NpcDialogueKind::Request => pools.request,
        NpcDialogueKind::Relationship => pools.relationship,
    };
    lines[ordinal % lines.len()]
}

/// Give a fired ceramic if present. Only authored liked forms grant +5 friendship.
pub fn give_ceramic(
    inventory: &mut Inventory,
    ceramics: &[CeramicItem],
    friendship: &mut NpcFriendship,
    character: NpcCharacter,
    id: CeramicObjectId,
) -> GiftResult {
    if !inventory.contains_ceramic(id) {
        return GiftResult::NotOwned;
    }
    let Some(item) = ceramics.iter().find(|item| item.id == id) else {
        return GiftResult::UnknownCeramic;
    };
    if item.state() != ProcessingState::Fired {
        return GiftResult::NotFired;
    }
    let liked_forms = NPC_LIKED_CERAMIC_FORMS
        .iter()
        .find(|(npc, _)| *npc == character)
        .map(|(_, forms)| *forms)
        .expect("every NPC has an authored gift preference");
    if !inventory.remove_ceramic(id) {
        return GiftResult::NotOwned;
    }
    if liked_forms.contains(&item.form()) {
        friendship.increase(5);
        GiftResult::Liked
    } else {
        GiftResult::NotLiked
    }
}

#[derive(Clone, Copy)]
struct SchedulePhase {
    state: NpcScheduleState,
    destination: NpcDestination,
    route: [(u8, u8); 3],
    route_len: usize,
    start_minute: f64,
    end_minute: f64,
}

fn schedule_phase(character: NpcCharacter, minute: f64) -> SchedulePhase {
    let (home, work) = match character {
        NpcCharacter::Baker => ((8, 12), (9, 12)),
        NpcCharacter::Carpenter => ((11, 12), (12, 12)),
        NpcCharacter::Merchant => ((14, 12), (15, 12)),
    };
    let social = SOCIAL_TILE;
    let stationary = [(0, 0); 3];
    let phases = [
        SchedulePhase {
            state: NpcScheduleState::Home,
            destination: NpcDestination::Home,
            route: stationary,
            route_len: 0,
            start_minute: 0.0,
            end_minute: 8.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Work,
            route: [home, (8, 13), work],
            route_len: 3,
            start_minute: 8.0 * 60.0,
            end_minute: 9.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Work,
            destination: NpcDestination::Work,
            route: stationary,
            route_len: 0,
            start_minute: 9.0 * 60.0,
            end_minute: 12.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Social,
            route: [work, (10, 14), social],
            route_len: 3,
            start_minute: 12.0 * 60.0,
            end_minute: 12.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Social,
            destination: NpcDestination::Social,
            route: stationary,
            route_len: 0,
            start_minute: 12.5 * 60.0,
            end_minute: 13.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Work,
            route: [social, (13, 14), work],
            route_len: 3,
            start_minute: 13.5 * 60.0,
            end_minute: 14.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Work,
            destination: NpcDestination::Work,
            route: stationary,
            route_len: 0,
            start_minute: 14.0 * 60.0,
            end_minute: 18.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Social,
            route: [work, (13, 14), social],
            route_len: 3,
            start_minute: 18.0 * 60.0,
            end_minute: 18.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Social,
            destination: NpcDestination::Social,
            route: stationary,
            route_len: 0,
            start_minute: 18.5 * 60.0,
            end_minute: 19.5 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Travel,
            destination: NpcDestination::Home,
            route: [social, (10, 14), home],
            route_len: 3,
            start_minute: 19.5 * 60.0,
            end_minute: 21.0 * 60.0,
        },
        SchedulePhase {
            state: NpcScheduleState::Home,
            destination: NpcDestination::Home,
            route: stationary,
            route_len: 0,
            start_minute: 21.0 * 60.0,
            end_minute: 24.0 * 60.0,
        },
    ];

    phases
        .into_iter()
        .find(|phase| minute >= phase.start_minute && minute < phase.end_minute)
        .unwrap_or(phases[0])
}

fn route_sample(
    route: &[(u8, u8); 3],
    route_len: usize,
    progress: f32,
) -> (crate::planet::SurfaceSample, usize) {
    let coordinates: Vec<_> = route[..route_len]
        .iter()
        .map(|&(x, y)| {
            let tile = TileCoordinate::new(PlanetFace::PositiveY, x, y)
                .expect("authored NPC route tiles remain within the planet face");
            let coordinate = PlanetCoordinate::new(tile, FaceOrientation::North);
            let tile = PlanetTile::new(coordinate.tile);
            (
                coordinate,
                sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
                    .expect("the configured planet radius is valid"),
            )
        })
        .collect();
    if coordinates.len() < 2 {
        return (coordinates[0].1, 0);
    }
    let scaled = progress.clamp(0.0, 1.0) * (coordinates.len() - 1) as f32;
    let index = (scaled.floor() as usize).min(coordinates.len() - 2);
    let fraction = scaled - index as f32;
    let (_, start_sample) = coordinates[index];
    let (_, end_sample) = coordinates[index + 1];
    let normal = start_sample
        .normal
        .lerp(end_sample.normal, fraction)
        .normalize();
    let height = coordinates[index]
        .1
        .height
        .lerp(coordinates[index + 1].1.height, fraction);
    (
        crate::planet::SurfaceSample {
            normal,
            height,
            position: normal * (DEFAULT_PLANET_RADIUS + height),
        },
        index + usize::from(fraction > 0.5),
    )
}

#[derive(Component)]
struct NpcDialogueText;

fn spawn_dialogue_ui(mut commands: Commands) {
    commands.spawn((
        NpcDialogueText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(48),
            left: px(16),
            ..default()
        },
    ));
}

fn handle_npc_talks(
    mut requests: MessageReader<InteractionRequested>,
    mut npcs: Query<(&NpcCharacter, &mut NpcFriendship)>,
    mut display: ResMut<NpcDialogueDisplay>,
) {
    for request in requests.read() {
        let Ok((character, mut friendship)) = npcs.get_mut(request.target) else {
            continue;
        };
        friendship.increase(1);
        let kind = if friendship.0 >= 50 {
            NpcDialogueKind::Relationship
        } else {
            NpcDialogueKind::Generic
        };
        display.0 = Some(NpcDialogueLine {
            speaker: *character,
            kind,
            text: select_dialogue(*character, kind, usize::from(friendship.0)),
        });
    }
}

fn handle_npc_gifts(
    mut requests: MessageReader<NpcGiftRequested>,
    mut inventory: ResMut<Inventory>,
    crafted: Res<CraftedCeramics>,
    mut npcs: Query<(&NpcCharacter, &mut NpcFriendship)>,
    mut display: ResMut<NpcDialogueDisplay>,
) {
    for request in requests.read() {
        let Ok((character, mut friendship)) = npcs.get_mut(request.npc) else {
            continue;
        };
        let result = give_ceramic(
            &mut inventory,
            &crafted.items,
            &mut friendship,
            *character,
            request.item,
        );
        if matches!(result, GiftResult::Liked | GiftResult::NotLiked) {
            let kind = if result == GiftResult::Liked && friendship.0 >= 50 {
                NpcDialogueKind::Relationship
            } else {
                NpcDialogueKind::Generic
            };
            display.0 = Some(NpcDialogueLine {
                speaker: *character,
                kind,
                text: select_dialogue(*character, kind, usize::from(friendship.0)),
            });
        }
    }
}

fn cancel_npc_dialogue(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut display: ResMut<NpcDialogueDisplay>,
) {
    if mouse.is_some_and(|input| input.just_pressed(MouseButton::Right)) {
        display.0 = None;
    }
}

fn update_dialogue_ui(
    display: Res<NpcDialogueDisplay>,
    mut labels: Query<&mut Text, With<NpcDialogueText>>,
) {
    if !display.is_changed() {
        return;
    }
    let text = display.0.as_ref().map_or_else(String::new, |line| {
        format!("{}: {}", npc_name(line.speaker), line.text)
    });
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}

const fn npc_name(character: NpcCharacter) -> &'static str {
    match character {
        NpcCharacter::Baker => "Baker",
        NpcCharacter::Carpenter => "Carpenter",
        NpcCharacter::Merchant => "Merchant",
    }
}

fn update_npc_schedules(
    clock: Res<GameClock>,
    mut npcs: Query<(&NpcCharacter, &mut Transform, &mut NpcScheduleDebug)>,
) {
    let minute = clock.minute_of_day();
    for (character, mut transform, mut debug) in &mut npcs {
        let phase = schedule_phase(*character, minute);
        let (sample, waypoint_index) = if phase.route_len == 0 {
            let tile = match phase.destination {
                NpcDestination::Home => match character {
                    NpcCharacter::Baker => (8, 12),
                    NpcCharacter::Carpenter => (11, 12),
                    NpcCharacter::Merchant => (14, 12),
                },
                NpcDestination::Work => match character {
                    NpcCharacter::Baker => (9, 12),
                    NpcCharacter::Carpenter => (12, 12),
                    NpcCharacter::Merchant => (15, 12),
                },
                NpcDestination::Social => SOCIAL_TILE,
            };
            route_sample(&[tile, tile, tile], 1, 0.0)
        } else {
            let progress =
                ((minute - phase.start_minute) / (phase.end_minute - phase.start_minute)) as f32;
            route_sample(&phase.route, phase.route_len, progress)
        };
        let direction = sample.normal;
        if let Some(next_transform) = surface_transform(
            SurfaceLocation::new(direction, 0.04),
            Vec3::ZERO,
            DEFAULT_PLANET_RADIUS,
            &crate::planet::SurfaceSample {
                height: sample.height,
                normal: direction,
                position: sample.position,
            },
            Vec3::X,
        ) {
            *transform = next_transform;
        }
        *debug = NpcScheduleDebug {
            state: phase.state,
            destination: phase.destination,
            waypoint_index,
        };
    }
}

fn spawn_npc_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(Capsule3d::new(0.38, 0.85));
    let head_mesh = meshes.add(Sphere::new(0.34));
    let chef_hat_mesh = meshes.add(Cylinder::new(0.22, 0.38));
    let toolbox_mesh = meshes.add(Cuboid::new(0.55, 0.48, 0.5));
    let merchant_hat_mesh = meshes.add(Sphere::new(0.19));
    let wood = materials.add(Color::srgb(0.48, 0.30, 0.16));
    let roof = materials.add(Color::srgb(0.69, 0.30, 0.18));
    let pale = materials.add(Color::srgb(0.86, 0.75, 0.54));
    let blue = materials.add(Color::srgb(0.24, 0.42, 0.60));

    for definition in NPCS {
        let body_material = materials.add(definition.body_color);
        let accessory_material = materials.add(definition.accessory_color);
        let character_transform = location_transform(definition.character_tile);
        let character = commands
            .spawn((
                Name::new(definition.character_name),
                definition.character,
                character_transform,
                Interactable::new(definition.interaction_prompt),
                NpcFriendship::default(),
                Visibility::default(),
                NpcScheduleDebug {
                    state: NpcScheduleState::Home,
                    destination: NpcDestination::Home,
                    waypoint_index: 0,
                },
            ))
            .id();
        if definition.character == NpcCharacter::Baker {
            commands.entity(character).insert((
                crate::npc_requests::NpcRequest::baker(),
                crate::npc_requests::BakerFinalOrder::default(),
            ));
        }
        commands.entity(character).with_children(|children| {
            children.spawn((
                Name::new(format!("{} body", definition.character_name)),
                Mesh3d(body_mesh.clone()),
                MeshMaterial3d(body_material),
                Transform::from_xyz(0.0, 0.72, 0.0),
            ));
            children.spawn((
                Name::new(format!("{} head", definition.character_name)),
                Mesh3d(head_mesh.clone()),
                MeshMaterial3d(materials.add(Color::srgb(0.83, 0.66, 0.52))),
                Transform::from_xyz(0.0, 1.48, 0.0),
            ));
            let (mesh, position, name) = match definition.character {
                NpcCharacter::Baker => {
                    (chef_hat_mesh.clone(), Vec3::new(0.0, 1.9, 0.0), "Chef hat")
                }
                NpcCharacter::Carpenter => (
                    toolbox_mesh.clone(),
                    Vec3::new(0.0, 0.75, 0.43),
                    "Toolbox backpack",
                ),
                NpcCharacter::Merchant => (
                    merchant_hat_mesh.clone(),
                    Vec3::new(0.0, 1.88, 0.0),
                    "Merchant hat",
                ),
            };
            children.spawn((
                Name::new(name),
                Mesh3d(mesh),
                MeshMaterial3d(accessory_material),
                Transform::from_translation(position),
            ));
        });

        let property_transform = location_transform(definition.property_tile);
        let mut property_bundle = commands.spawn((
            Name::new(definition.property_name),
            definition.property,
            property_transform,
            Interactable::new(format!("Visit the {}", definition.property_name))
                .with_pick_radius(2.0),
            Visibility::default(),
        ));
        if definition.property == NpcProperty::Bakery {
            property_bundle.insert(crate::npc_property_upgrade::BakeryUpgrade::default());
        }
        let property = property_bundle.id();
        commands.entity(property).with_children(|children| {
            let (wall_material, roof_material) = match definition.property {
                NpcProperty::Bakery => (pale.clone(), roof.clone()),
                NpcProperty::Workshop => (wood.clone(), roof.clone()),
                NpcProperty::GeneralStore => (pale.clone(), blue.clone()),
            };
            // A simple, open-fronted shell: side walls, rear wall, and a pitched-roof cue.
            children.spawn((
                Name::new(format!("{} walls", definition.property_name)),
                Mesh3d(meshes.add(Cuboid::new(2.2, 1.8, 0.16))),
                MeshMaterial3d(wall_material.clone()),
                Transform::from_xyz(0.0, 0.9, -0.9),
            ));
            for x in [-1.02, 1.02] {
                children.spawn((
                    Name::new(format!("{} side wall", definition.property_name)),
                    Mesh3d(meshes.add(Cuboid::new(0.16, 1.8, 1.8))),
                    MeshMaterial3d(wall_material.clone()),
                    Transform::from_xyz(x, 0.9, 0.0),
                ));
            }
            children.spawn((
                Name::new(format!("{} roof", definition.property_name)),
                Mesh3d(meshes.add(Cuboid::new(2.55, 0.18, 2.35))),
                MeshMaterial3d(roof_material),
                Transform::from_xyz(0.0, 1.92, -0.02).with_rotation(Quat::from_rotation_z(0.12)),
            ));
            if definition.property == NpcProperty::Bakery {
                // Two deterministic counter positions: one cup per slot, no production clutter.
                for (index, x) in [-0.42, 0.42].into_iter().enumerate() {
                    children.spawn((
                        Name::new(format!("Bakery ceramic slot {index}")),
                        crate::npc_placement_slots::NpcPlacementSlot::bakery(index),
                        Transform::from_xyz(x, 0.12, 0.62),
                    ));
                }
            } else if definition.property == NpcProperty::Workshop {
                // The only physical NPC production consequence has one authored placement slot.
                children.spawn((
                    Name::new("Carpenter world-event slot 0"),
                    crate::npc_placement_slots::NpcPropertyEventSlot::workshop(0),
                    Transform::from_xyz(0.0, 0.0, -1.25),
                ));
            }
        });

        if definition.character == NpcCharacter::Merchant {
            commands.entity(character).insert(Merchant);
        }
    }
}

fn location_transform(tile: (u8, u8)) -> Transform {
    let coordinate = TileCoordinate::new(PlanetFace::PositiveY, tile.0, tile.1)
        .expect("NPC location tiles are within the authored planet face");
    let surface = sample_tile_surface(&PlanetTile::new(coordinate), DEFAULT_PLANET_RADIUS)
        .expect("the configured planet radius is valid");
    surface_transform(
        SurfaceLocation::new(surface.normal, 0.04),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &surface,
        Vec3::X,
    )
    .expect("NPC locations have valid tangent orientations")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fired_ceramic(id: u64, form: CeramicForm) -> CeramicItem {
        crate::ceramics::CeramicItemTemplate {
            form,
            clay: crate::ceramics::ClayMaterial::Common,
            glaze: crate::ceramics::Glaze::None,
            state: ProcessingState::Fired,
        }
        .instantiate(CeramicObjectId(id))
    }

    #[test]
    fn every_npc_has_the_required_authored_dialogue_and_explicit_gift_preferences() {
        for character in [
            NpcCharacter::Baker,
            NpcCharacter::Carpenter,
            NpcCharacter::Merchant,
        ] {
            let pools = dialogue_pools(character);
            assert!(pools.generic.len() >= 5);
            assert!(pools.request.len() >= 3);
            assert!(pools.relationship.len() >= 3);
            for kind in [
                NpcDialogueKind::Generic,
                NpcDialogueKind::Request,
                NpcDialogueKind::Relationship,
            ] {
                assert!(!select_dialogue(character, kind, usize::MAX).is_empty());
            }
        }
        assert_eq!(NPC_LIKED_CERAMIC_FORMS.len(), 3);
        assert!(
            NPC_LIKED_CERAMIC_FORMS
                .iter()
                .all(|(_, forms)| !forms.is_empty())
        );
    }

    #[test]
    fn friendship_increments_and_caps_at_one_hundred() {
        let mut friendship = NpcFriendship(99);
        friendship.increase(1);
        assert_eq!(friendship.0, 100);
        friendship.increase(5);
        assert_eq!(friendship.0, 100);
        friendship.increase(0);
        assert_eq!(friendship.0, 100);
    }

    #[test]
    fn liked_fired_gifts_are_consumed_and_add_five_friendship() {
        let mut inventory = Inventory::default();
        let item = fired_ceramic(1, CeramicForm::Cup);
        assert!(inventory.add_ceramic(item.id));
        let mut friendship = NpcFriendship(97);
        assert_eq!(
            give_ceramic(
                &mut inventory,
                &[item],
                &mut friendship,
                NpcCharacter::Baker,
                item.id
            ),
            GiftResult::Liked
        );
        assert_eq!(friendship.0, 100);
        assert!(!inventory.contains_ceramic(item.id));
    }

    #[test]
    fn liked_gift_rules_are_npc_specific_and_reject_unfinished_or_unowned_items() {
        let mut inventory = Inventory::default();
        let vase = fired_ceramic(2, CeramicForm::Vase);
        inventory.add_ceramic(vase.id);
        let mut friendship = NpcFriendship::default();
        assert_eq!(
            give_ceramic(
                &mut inventory,
                &[vase],
                &mut friendship,
                NpcCharacter::Baker,
                vase.id
            ),
            GiftResult::NotLiked
        );
        assert_eq!(friendship.0, 0);
        assert!(!inventory.contains_ceramic(vase.id));

        let unfired = CeramicItem {
            template: crate::ceramics::CeramicItemTemplate {
                state: ProcessingState::Greenware,
                ..vase.template
            },
            ..vase
        };
        inventory.add_ceramic(unfired.id);
        assert_eq!(
            give_ceramic(
                &mut inventory,
                &[unfired],
                &mut friendship,
                NpcCharacter::Merchant,
                unfired.id
            ),
            GiftResult::NotFired
        );
        assert!(inventory.contains_ceramic(unfired.id));
        let unowned = fired_ceramic(3, CeramicForm::Cup);
        assert_eq!(
            give_ceramic(
                &mut inventory,
                &[unowned],
                &mut friendship,
                NpcCharacter::Merchant,
                unowned.id
            ),
            GiftResult::NotOwned
        );
    }

    #[test]
    fn right_click_cancels_the_active_dialogue() {
        let mut app = App::new();
        app.init_resource::<NpcDialogueDisplay>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, cancel_npc_dialogue);
        app.world_mut().resource_mut::<NpcDialogueDisplay>().0 = Some(NpcDialogueLine {
            speaker: NpcCharacter::Baker,
            kind: NpcDialogueKind::Generic,
            text: "Hello there.",
        });
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        app.update();
        assert_eq!(app.world().resource::<NpcDialogueDisplay>().0, None);
    }

    #[test]
    fn talking_increases_friendship_and_presents_authored_dialogue() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(GameClock::default());
        app.add_plugins(NpcPlugin);
        app.update();
        let entity = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<NpcCharacter>>();
            query.iter(world).next().unwrap()
        };
        app.world_mut()
            .write_message(InteractionRequested { target: entity });
        app.update();
        let world = app.world();
        assert_eq!(world.get::<NpcFriendship>(entity), Some(&NpcFriendship(1)));
        let line = world.resource::<NpcDialogueDisplay>().0.as_ref().unwrap();
        assert_eq!(line.kind, NpcDialogueKind::Generic);
        assert!(dialogue_pools(line.speaker).generic.contains(&line.text));
    }

    #[test]
    fn liked_gift_event_updates_friendship_and_presents_authored_dialogue() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(GameClock::default());
        app.add_plugins(NpcPlugin);
        app.update();

        let item = fired_ceramic(42, CeramicForm::Cup);
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_ceramic(item.id);
        app.world_mut()
            .resource_mut::<CraftedCeramics>()
            .items
            .push(item);
        let npc = app
            .world_mut()
            .spawn((NpcCharacter::Baker, NpcFriendship(98)))
            .id();
        app.world_mut()
            .write_message(NpcGiftRequested { npc, item: item.id });
        app.update();

        assert_eq!(
            app.world().get::<NpcFriendship>(npc),
            Some(&NpcFriendship(100))
        );
        let line = app
            .world()
            .resource::<NpcDialogueDisplay>()
            .0
            .as_ref()
            .unwrap();
        assert_eq!(line.speaker, NpcCharacter::Baker);
        assert_eq!(line.kind, NpcDialogueKind::Relationship);
        assert!(
            dialogue_pools(line.speaker)
                .relationship
                .contains(&line.text)
        );
        assert!(
            !app.world()
                .resource::<Inventory>()
                .contains_ceramic(item.id)
        );
    }

    #[test]
    fn schedule_boundaries_are_deterministic_and_expose_expected_destinations() {
        let cases = [
            (0.0, NpcScheduleState::Home, NpcDestination::Home),
            (8.0 * 60.0, NpcScheduleState::Travel, NpcDestination::Work),
            (9.0 * 60.0, NpcScheduleState::Work, NpcDestination::Work),
            (
                12.0 * 60.0,
                NpcScheduleState::Travel,
                NpcDestination::Social,
            ),
            (
                12.5 * 60.0,
                NpcScheduleState::Social,
                NpcDestination::Social,
            ),
            (13.5 * 60.0, NpcScheduleState::Travel, NpcDestination::Work),
            (14.0 * 60.0, NpcScheduleState::Work, NpcDestination::Work),
            (
                18.0 * 60.0,
                NpcScheduleState::Travel,
                NpcDestination::Social,
            ),
            (
                18.5 * 60.0,
                NpcScheduleState::Social,
                NpcDestination::Social,
            ),
            (19.5 * 60.0, NpcScheduleState::Travel, NpcDestination::Home),
            (21.0 * 60.0, NpcScheduleState::Home, NpcDestination::Home),
        ];
        for character in [
            NpcCharacter::Baker,
            NpcCharacter::Carpenter,
            NpcCharacter::Merchant,
        ] {
            for (minute, expected_state, expected_destination) in cases {
                let schedule = schedule_phase(character, minute);
                assert_eq!(schedule.state, expected_state);
                assert_eq!(schedule.destination, expected_destination);
                assert_eq!(schedule_phase(character, minute).state, schedule.state);
            }
        }
    }

    #[test]
    fn authored_travel_waypoints_interpolate_on_sphere_and_cube_seam() {
        let character = NpcCharacter::Baker;
        let phase = schedule_phase(character, 8.0 * 60.0);
        let (start, _) = route_sample(&phase.route, phase.route_len, 0.0);
        let (middle, _) = route_sample(&phase.route, phase.route_len, 0.5);
        let (end, _) = route_sample(&phase.route, phase.route_len, 1.0);
        assert!((start.position.length() - DEFAULT_PLANET_RADIUS).abs() < 0.1);
        assert!((end.position.length() - DEFAULT_PLANET_RADIUS).abs() < 0.1);
        assert!(middle.normal.distance(start.normal) > 0.0);

        let edge = TileCoordinate::new(PlanetFace::PositiveY, 23, 12).unwrap();
        let across = crate::planet::move_coordinate(
            PlanetCoordinate::new(edge, FaceOrientation::East),
            crate::planet::Direction::East,
        );
        assert_ne!(across.tile.face(), edge.face());
        let edge_sample =
            sample_tile_surface(&PlanetTile::new(edge), DEFAULT_PLANET_RADIUS).unwrap();
        let across_sample =
            sample_tile_surface(&PlanetTile::new(across.tile), DEFAULT_PLANET_RADIUS).unwrap();
        assert!(edge_sample.normal.distance(across_sample.normal) < 0.2);
    }

    #[test]
    fn headless_world_contains_exactly_three_interactable_surface_aligned_npcs_and_properties() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(GameClock::default());
        app.add_plugins(NpcPlugin);
        app.update();

        let world = app.world_mut();
        let mut characters = world.query::<(
            &Name,
            &NpcCharacter,
            &Transform,
            &Interactable,
            &NpcScheduleDebug,
        )>();
        let characters: Vec<_> = characters.iter(world).collect();
        assert_eq!(characters.len(), 3);
        for expected in ["Baker", "Carpenter", "Merchant"] {
            assert_eq!(
                characters
                    .iter()
                    .filter(|(name, _, _, _, _)| name.as_str() == expected)
                    .count(),
                1
            );
        }
        for (_, _, transform, interactable, schedule) in characters {
            assert!(interactable.range > 0.0);
            assert_eq!(schedule.state, NpcScheduleState::Home);
            assert_eq!(schedule.destination, NpcDestination::Home);
            let up = transform.rotation * Vec3::Y;
            assert!(up.dot(transform.translation.normalize()) > 0.999);
        }

        let mut properties = world.query::<(&Name, &NpcProperty, &Transform, &Interactable)>();
        let properties: Vec<_> = properties.iter(world).collect();
        assert_eq!(properties.len(), 3);
        for expected in ["Bakery", "Workshop", "General Store"] {
            assert_eq!(
                properties
                    .iter()
                    .filter(|(name, _, _, _)| name.as_str() == expected)
                    .count(),
                1
            );
        }
        for (_, _, transform, interactable) in properties {
            assert!(interactable.range > 0.0);
            let up = transform.rotation * Vec3::Y;
            assert!(up.dot(transform.translation.normalize()) > 0.999);
        }

        let mut merchants = world.query_filtered::<Entity, With<Merchant>>();
        assert_eq!(merchants.iter(world).count(), 1);
    }
}
