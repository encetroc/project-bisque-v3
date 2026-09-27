//! Single-slot persistence for the ceramic planet prototype.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

use crate::{
    ceramic_visuals::CeramicVisual,
    ceramics::CeramicItem,
    drying_rack::{DRYING_RACK_MAX_CAPACITY, DryingJob, DryingRack},
    economy::Wallet,
    game_clock::{GameClock, MINUTES_PER_GAME_DAY},
    inventory::{CeramicObjectId, INVENTORY_SLOT_COUNT, Inventory, InventorySlot},
    kiln::{FiringJob, KILN_MAX_CAPACITY, Kiln},
    npc_placement_slots::{NpcPlacementSlot, NpcPropertyEventSlot},
    npc_production::{AppliedNpcPropertyEvents, NpcPropertyEvent},
    npc_property_upgrade::BakeryUpgrade,
    npc_requests::{BakerFinalOrder, NpcRequest, NpcRequestState},
    npcs::{NpcCharacter, NpcFriendship, NpcProperty},
    objectives::ObjectiveProgress,
    placement::PlayerPlacedObject,
    planet::{ResourceType, TileCoordinate},
    player_movement::SurfacePlayer,
    resource_nodes::{GatheredRedClay, RedClayDiscovery, ResourceNode},
    workbench::{CraftedCeramics, Workbench},
};

const SAVE_PATH: &str = "save.json";
const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MachineJobSave {
    pub object: CeramicObjectId,
    pub started_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcSave {
    pub character: NpcCharacter,
    pub friendship: u8,
    pub request_state: Option<NpcRequestState>,
    #[serde(default)]
    pub baker_final_order_state: Option<NpcRequestState>,
    pub bakery_upgrade_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementSlotSave {
    pub property: NpcProperty,
    pub index: usize,
    pub ceramic: Option<CeramicItem>,
    pub event: Option<NpcPropertyEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerPlacedSave {
    pub item: CeramicItem,
    pub direction: [f32; 3],
    pub altitude: f32,
    pub rotation: f32,
}

/// Versioned representation of one save slot. No ECS entity IDs are persisted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSave {
    pub version: u32,
    pub inventory: Vec<InventorySlot>,
    pub coins: u64,
    pub day: u64,
    pub minute_of_day: f64,
    pub ceramics: Vec<CeramicItem>,
    pub next_ceramic_id: u64,
    pub player_direction: [f32; 3],
    pub player_heading: [f32; 3],
    pub player_altitude: f32,
    pub workbench_upgraded: bool,
    pub drying_upgraded: bool,
    pub drying_jobs: [Option<MachineJobSave>; DRYING_RACK_MAX_CAPACITY],
    pub kiln_upgraded: bool,
    pub firing_jobs: [Option<MachineJobSave>; KILN_MAX_CAPACITY],
    pub npcs: Vec<NpcSave>,
    pub gathered_resources: Vec<TileCoordinate>,
    pub player_placements: Vec<PlayerPlacedSave>,
    pub placement_slots: Vec<PlacementSlotSave>,
    pub red_clay_discovered: bool,
    #[serde(default)]
    pub objectives: ObjectiveProgress,
}

impl GameSave {
    pub fn clean() -> Self {
        let clock = GameClock::default();
        Self {
            version: SAVE_VERSION,
            inventory: vec![InventorySlot::Empty; INVENTORY_SLOT_COUNT],
            coins: 0,
            day: clock.day(),
            minute_of_day: clock.minute_of_day(),
            ceramics: Vec::new(),
            next_ceramic_id: 0,
            player_direction: Vec3::Y.to_array(),
            player_heading: Vec3::X.to_array(),
            player_altitude: 1.6,
            workbench_upgraded: false,
            drying_upgraded: false,
            drying_jobs: [None; DRYING_RACK_MAX_CAPACITY],
            kiln_upgraded: false,
            firing_jobs: [None; KILN_MAX_CAPACITY],
            npcs: Vec::new(),
            gathered_resources: Vec::new(),
            player_placements: Vec::new(),
            placement_slots: Vec::new(),
            red_clay_discovered: false,
            objectives: ObjectiveProgress::default(),
        }
    }

    pub fn validate(&self) -> bool {
        self.version == SAVE_VERSION
            && self.inventory.len() == INVENTORY_SLOT_COUNT
            && self.inventory.iter().all(|slot| match slot {
                InventorySlot::ResourceStack { count, .. } => (1..=99).contains(count),
                _ => true,
            })
            && self.day > 0
            && self.minute_of_day.is_finite()
            && (0.0..MINUTES_PER_GAME_DAY).contains(&self.minute_of_day)
            && valid_vector(self.player_direction)
            && valid_vector(self.player_heading)
            && self.player_altitude.is_finite()
            && unique(self.ceramics.iter().map(|item| item.id))
            && unique(self.npcs.iter().map(|npc| npc.character))
            && self.npcs.iter().all(|npc| npc.friendship <= 100)
            && unique(self.gathered_resources.iter().copied())
            && self.gathered_resources.iter().all(|coordinate| {
                coordinate.x() < crate::planet::TILES_PER_FACE
                    && coordinate.y() < crate::planet::TILES_PER_FACE
            })
            && unique(
                self.placement_slots
                    .iter()
                    .map(|slot| (slot.property, slot.index)),
            )
            && unique(self.placement_slots.iter().filter_map(|slot| slot.event))
            && self.placement_slots.iter().all(|slot| {
                !(slot.ceramic.is_some() && slot.event.is_some())
                    && (slot.ceramic.is_none() || slot.property == NpcProperty::Bakery)
                    && (slot.event.is_none()
                        || (slot.property == NpcProperty::Workshop && slot.index == 0))
                    && slot
                        .ceramic
                        .is_none_or(|item| self.ceramics.contains(&item))
            })
            && self.player_placements.iter().all(|placed| {
                self.ceramics.contains(&placed.item)
                    && valid_vector(placed.direction)
                    && placed.altitude.is_finite()
                    && placed.rotation.is_finite()
            })
            && self.drying_jobs.iter().flatten().all(|job| {
                job.started_at.is_finite() && self.ceramics.iter().any(|item| item.id == job.object)
            })
            && self.firing_jobs.iter().flatten().all(|job| {
                job.started_at.is_finite() && self.ceramics.iter().any(|item| item.id == job.object)
            })
            && self.next_ceramic_id
                >= self
                    .ceramics
                    .iter()
                    .map(|item| item.id.0)
                    .max()
                    .unwrap_or(0)
            && self.objectives.tracked_cup.is_none_or(|id| {
                self.ceramics
                    .iter()
                    .any(|item| item.id == id && item.form() == crate::ceramics::CeramicForm::Cup)
            })
            && self.unique_owned_ceramics()
    }

    fn unique_owned_ceramics(&self) -> bool {
        let ids = self
            .inventory
            .iter()
            .filter_map(|slot| match slot {
                InventorySlot::Ceramic(id) => Some(*id),
                _ => None,
            })
            .chain(self.drying_jobs.iter().flatten().map(|job| job.object))
            .chain(self.firing_jobs.iter().flatten().map(|job| job.object))
            .chain(self.player_placements.iter().map(|placed| placed.item.id))
            .chain(
                self.placement_slots
                    .iter()
                    .filter_map(|slot| slot.ceramic.map(|item| item.id)),
            );
        let ids = ids.collect::<Vec<_>>();
        let known: HashSet<_> = self.ceramics.iter().map(|item| item.id).collect();
        unique(ids.iter().copied()) && ids.iter().all(|id| known.contains(id))
    }

    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Invalid, incompatible, absent, or malformed data is treated as a new game.
    pub fn decode(json: &str) -> Option<Self> {
        serde_json::from_str::<Self>(json)
            .ok()
            .filter(GameSave::validate)
    }
}

fn unique<T: Eq + std::hash::Hash>(items: impl IntoIterator<Item = T>) -> bool {
    let values: Vec<_> = items.into_iter().collect();
    values.iter().collect::<HashSet<_>>().len() == values.len()
}

fn valid_vector(vector: [f32; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
        && vector.iter().map(|value| value * value).sum::<f32>() > 1.0e-6
}

/// F5 saves the single slot, F9 loads it; startup loads a valid existing slot.
pub struct SaveGamePlugin;

impl Plugin for SaveGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, load_save_on_startup)
            .add_systems(Update, save_load_hotkeys);
    }
}

fn load_save_on_startup(world: &mut World) {
    if let Some(save) = read_slot(Path::new(SAVE_PATH)) {
        apply_save(world, &save);
    }
}

fn save_load_hotkeys(world: &mut World) {
    let Some(keyboard) = world.get_resource::<ButtonInput<KeyCode>>() else {
        return;
    };
    let (save_pressed, load_pressed) = (
        keyboard.just_pressed(KeyCode::F5),
        keyboard.just_pressed(KeyCode::F9),
    );
    if save_pressed {
        let save = capture_save(world);
        let _ = write_slot(Path::new(SAVE_PATH), &save);
    }
    if load_pressed && let Some(save) = read_slot(Path::new(SAVE_PATH)) {
        apply_save(world, &save);
    }
}

fn read_slot(path: &Path) -> Option<GameSave> {
    let json = std::fs::read_to_string(path).ok()?;
    GameSave::decode(&json)
}

fn write_slot(path: &Path, save: &GameSave) -> std::io::Result<()> {
    if !save.validate() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "refusing to write invalid game state",
        ));
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, save.encode().map_err(std::io::Error::other)?)?;
    std::fs::rename(temporary, path)
}

fn capture_save(world: &mut World) -> GameSave {
    let mut save = GameSave::clean();
    if let Some(value) = world.get_resource::<Inventory>() {
        save.inventory = value.slots().to_vec();
    }
    if let Some(value) = world.get_resource::<Wallet>() {
        save.coins = value.coins;
    }
    if let Some(value) = world.get_resource::<GameClock>() {
        save.day = value.day();
        save.minute_of_day = value.minute_of_day();
    }
    if let Some(value) = world.get_resource::<CraftedCeramics>() {
        save.ceramics.clone_from(&value.items);
        save.next_ceramic_id = value.next_id();
    }
    if let Some(value) = world.get_resource::<RedClayDiscovery>() {
        save.red_clay_discovered = value.discovered;
    }
    if let Some(value) = world.get_resource::<ObjectiveProgress>() {
        save.objectives = *value;
    }
    {
        let mut q = world.query::<&SurfacePlayer>();
        if let Some(player) = q.iter(world).next() {
            save.player_direction = player.location.direction.to_array();
            save.player_heading = player.heading.to_array();
            save.player_altitude = player.location.altitude;
        }
    }
    {
        let mut q = world.query::<&Workbench>();
        save.workbench_upgraded = q.iter(world).any(|value| value.upgraded);
    }
    {
        let mut q = world.query::<&DryingRack>();
        if let Some(rack) = q.iter(world).next() {
            save.drying_upgraded = rack.upgraded;
            save.drying_jobs = rack.slots().map(|job| {
                job.map(|job| MachineJobSave {
                    object: job.object,
                    started_at: job.started_at(),
                })
            });
        }
    }
    {
        let mut q = world.query::<&Kiln>();
        if let Some(kiln) = q.iter(world).next() {
            save.kiln_upgraded = kiln.upgraded;
            save.firing_jobs = kiln.slots().map(|job| {
                job.map(|job| MachineJobSave {
                    object: job.object,
                    started_at: job.started_at(),
                })
            });
        }
    }
    {
        let mut q = world.query::<(
            &NpcCharacter,
            &NpcFriendship,
            Option<&NpcRequest>,
            Option<&BakerFinalOrder>,
        )>();
        save.npcs = q
            .iter(world)
            .map(|(character, friendship, request, final_order)| NpcSave {
                character: *character,
                friendship: friendship.0,
                request_state: request.map(|request| request.state),
                baker_final_order_state: final_order.map(|order| order.state),
                bakery_upgrade_complete: false,
            })
            .collect();
        let mut q = world.query::<(&NpcProperty, &BakeryUpgrade)>();
        for (property, upgrade) in q.iter(world) {
            if *property == NpcProperty::Bakery
                && let Some(baker) = save
                    .npcs
                    .iter_mut()
                    .find(|npc| npc.character == NpcCharacter::Baker)
            {
                baker.bakery_upgrade_complete = upgrade.complete;
            }
        }
    }
    {
        let mut q = world.query::<(&ResourceNode, Option<&GatheredRedClay>)>();
        save.gathered_resources = q
            .iter(world)
            .filter_map(|(node, gathered)| gathered.map(|_| node.coordinate))
            .collect();
    }
    {
        let mut q = world.query::<&PlayerPlacedObject>();
        save.player_placements = q
            .iter(world)
            .map(|placed| PlayerPlacedSave {
                item: placed.item,
                direction: placed.location.direction.to_array(),
                altitude: placed.location.altitude,
                rotation: placed.rotation,
            })
            .collect();
    }
    {
        let mut q = world.query::<(&NpcPlacementSlot, &ChildOf)>();
        save.placement_slots = q
            .iter(world)
            .filter_map(|(slot, parent)| {
                Some(PlacementSlotSave {
                    property: *world.get::<NpcProperty>(parent.parent())?,
                    index: slot.index,
                    ceramic: slot.occupied_by,
                    event: None,
                })
            })
            .collect();
        let mut q = world.query::<(&NpcPropertyEventSlot, &ChildOf)>();
        for (slot, parent) in q.iter(world) {
            if let Some(property) = world.get::<NpcProperty>(parent.parent()) {
                save.placement_slots.push(PlacementSlotSave {
                    property: *property,
                    index: slot.index,
                    ceramic: None,
                    event: slot.occupied_by,
                });
            }
        }
    }
    save.npcs.sort_by_key(|npc| match npc.character {
        NpcCharacter::Baker => 0,
        NpcCharacter::Carpenter => 1,
        NpcCharacter::Merchant => 2,
    });
    save.placement_slots.sort_by_key(|slot| {
        (
            match slot.property {
                NpcProperty::Bakery => 0,
                NpcProperty::Workshop => 1,
                NpcProperty::GeneralStore => 2,
            },
            slot.index,
        )
    });
    save
}

fn apply_save(world: &mut World, save: &GameSave) {
    if !save.validate() {
        return;
    }
    if let Some(mut inventory) = world.get_resource_mut::<Inventory>()
        && let Ok(slots) = save.inventory.clone().try_into()
    {
        inventory.restore_slots(slots);
    }
    if let Some(mut wallet) = world.get_resource_mut::<Wallet>() {
        wallet.coins = save.coins;
    }
    if let Some(mut clock) = world.get_resource_mut::<GameClock>() {
        clock.restore(save.day, save.minute_of_day);
    }
    if let Some(mut ceramics) = world.get_resource_mut::<CraftedCeramics>() {
        ceramics.restore(save.ceramics.clone(), save.next_ceramic_id);
    }
    if let Some(mut discovery) = world.get_resource_mut::<RedClayDiscovery>() {
        discovery.discovered = save.red_clay_discovered;
    }
    if let Some(mut objectives) = world.get_resource_mut::<ObjectiveProgress>() {
        *objectives = save.objectives;
    }
    {
        let mut q = world.query::<(&mut SurfacePlayer, &mut Transform)>();
        for (mut player, mut transform) in q.iter_mut(world) {
            player.location.direction = Vec3::from_array(save.player_direction);
            player.location.altitude = save.player_altitude;
            player.heading = Vec3::from_array(save.player_heading);
            let sample = crate::planet::SurfaceSample {
                position: player.location.direction
                    * (player.planet_radius + player.terrain_height),
                height: player.terrain_height,
                normal: player.location.direction,
            };
            if let Some(restored) = crate::surface_transform::surface_transform(
                player.location,
                Vec3::ZERO,
                player.planet_radius,
                &sample,
                player.heading,
            ) {
                *transform = restored;
            }
        }
    }
    {
        let mut q = world.query::<&mut Workbench>();
        for mut value in q.iter_mut(world) {
            value.upgraded = save.workbench_upgraded;
        }
    }
    {
        let mut q = world.query::<&mut DryingRack>();
        for mut value in q.iter_mut(world) {
            value.restore(
                save.drying_jobs
                    .map(|job| job.map(|job| DryingJob::new(job.object, job.started_at))),
                save.drying_upgraded,
            );
        }
    }
    {
        let mut q = world.query::<&mut Kiln>();
        for mut value in q.iter_mut(world) {
            value.restore(
                save.firing_jobs
                    .map(|job| job.map(|job| FiringJob::new(job.object, job.started_at))),
                save.kiln_upgraded,
            );
        }
    }
    for npc in &save.npcs {
        let mut q = world.query::<(
            &NpcCharacter,
            &mut NpcFriendship,
            Option<&mut NpcRequest>,
            Option<&mut BakerFinalOrder>,
        )>();
        for (character, mut friendship, request, final_order) in q.iter_mut(world) {
            if *character == npc.character {
                friendship.0 = npc.friendship;
                if let (Some(state), Some(mut request)) = (npc.request_state, request) {
                    request.state = state;
                }
                if let (Some(state), Some(mut final_order)) =
                    (npc.baker_final_order_state, final_order)
                {
                    final_order.state = state;
                }
            }
        }
    }
    {
        let mut q = world.query::<(&NpcProperty, &mut BakeryUpgrade)>();
        for (property, mut upgrade) in q.iter_mut(world) {
            if *property == NpcProperty::Bakery {
                upgrade.complete = save
                    .npcs
                    .iter()
                    .find(|npc| npc.character == NpcCharacter::Baker)
                    .is_some_and(|npc| npc.bakery_upgrade_complete);
            }
        }
    }
    {
        let gathered: HashSet<_> = save.gathered_resources.iter().copied().collect();
        let nodes = {
            let mut q = world.query::<(Entity, &ResourceNode, Option<&GatheredRedClay>)>();
            q.iter(world)
                .map(|(entity, node, marker)| {
                    (
                        entity,
                        node.coordinate,
                        node.biome == crate::planet::Biome::RedHighlands
                            && node.resource_type == ResourceType::RedClay,
                        marker.is_some(),
                    )
                })
                .collect::<Vec<_>>()
        };
        for (entity, coordinate, red_clay, was_gathered) in nodes {
            let should_gather = gathered.contains(&coordinate);
            if should_gather && !was_gathered {
                world
                    .entity_mut(entity)
                    .insert(GatheredRedClay)
                    .remove::<crate::interaction::Interactable>();
            } else if !should_gather && was_gathered {
                world.entity_mut(entity).remove::<GatheredRedClay>();
                if red_clay {
                    world
                        .entity_mut(entity)
                        .insert(crate::interaction::Interactable::new("Gather red clay"));
                }
            }
        }
    }
    restore_npc_slots(world, &save.placement_slots);
    if let Some(mut applied) = world.get_resource_mut::<AppliedNpcPropertyEvents>() {
        applied.restore(save.placement_slots.iter().filter_map(|slot| slot.event));
    }
    restore_player_placements(world, &save.player_placements);
}

fn restore_npc_slots(world: &mut World, slots: &[PlacementSlotSave]) {
    let ceramic = {
        let mut q = world.query::<(Entity, &NpcPlacementSlot, &ChildOf)>();
        q.iter(world)
            .filter_map(|(entity, current, parent)| {
                let property = *world.get::<NpcProperty>(parent.parent())?;
                let mut updated = *current;
                updated.occupied_by = slots
                    .iter()
                    .find(|saved| saved.property == property && saved.index == current.index)
                    .and_then(|saved| saved.ceramic);
                Some((entity, updated))
            })
            .collect::<Vec<_>>()
    };
    for (entity, slot) in ceramic {
        world.entity_mut(entity).insert(slot);
    }
    let events = {
        let mut q = world.query::<(Entity, &NpcPropertyEventSlot, &ChildOf)>();
        q.iter(world)
            .filter_map(|(entity, current, parent)| {
                let property = *world.get::<NpcProperty>(parent.parent())?;
                let mut updated = *current;
                updated.occupied_by = slots
                    .iter()
                    .find(|saved| saved.property == property && saved.index == current.index)
                    .and_then(|saved| saved.event);
                Some((entity, updated))
            })
            .collect::<Vec<_>>()
    };
    for (entity, slot) in events {
        world.entity_mut(entity).insert(slot);
    }
    restore_bakery_slots(world, slots);
    sync_npc_ceramic_visuals(world);
    restore_carpenter_event_visuals(world);
}

fn restore_bakery_slots(world: &mut World, saved_slots: &[PlacementSlotSave]) {
    let upgrade_complete = {
        let mut query = world.query::<(&NpcProperty, &BakeryUpgrade)>();
        query
            .iter(world)
            .any(|(property, upgrade)| *property == NpcProperty::Bakery && upgrade.complete)
    };
    let bakery = {
        let mut query = world.query::<(Entity, &NpcProperty)>();
        query
            .iter(world)
            .find_map(|(entity, property)| (*property == NpcProperty::Bakery).then_some(entity))
    };
    let Some(bakery) = bakery else {
        return;
    };
    let existing = {
        let mut query = world.query::<(Entity, &NpcPlacementSlot, &ChildOf)>();
        query
            .iter(world)
            .filter(|(_, _, parent)| parent.parent() == bakery)
            .map(|(entity, slot, _)| (entity, slot.index))
            .collect::<Vec<_>>()
    };
    let required_count = if upgrade_complete {
        saved_slots
            .iter()
            .filter(|slot| slot.property == NpcProperty::Bakery)
            .map(|slot| slot.index + 1)
            .max()
            .unwrap_or(8)
            .max(8)
    } else {
        2
    };
    let obsolete = existing
        .iter()
        .filter(|(_, index)| *index >= required_count)
        .map(|(entity, _)| *entity)
        .collect::<Vec<_>>();
    for entity in obsolete {
        world.entity_mut(entity).despawn();
    }
    let positions = crate::npc_property_upgrade::bakery_slot_positions();
    for (index, position) in positions.iter().enumerate().take(required_count) {
        let item = saved_slots
            .iter()
            .find(|slot| slot.property == NpcProperty::Bakery && slot.index == index)
            .and_then(|slot| slot.ceramic);
        if let Some((entity, _)) = existing
            .iter()
            .find(|(_, existing_index)| *existing_index == index)
        {
            if upgrade_complete && let Some(mut transform) = world.get_mut::<Transform>(*entity) {
                transform.translation = *position;
            }
        } else {
            let mut slot = NpcPlacementSlot::bakery(index);
            slot.occupied_by = item;
            world.spawn((
                Name::new(format!("Bakery ceramic slot {index}")),
                slot,
                Transform::from_translation(*position),
                ChildOf(bakery),
            ));
        }
    }
    rebuild_bakery_expansion(world, bakery, upgrade_complete);
}

fn rebuild_bakery_expansion(world: &mut World, bakery: Entity, enabled: bool) {
    let old = {
        let mut query =
            world.query_filtered::<Entity, With<crate::npc_property_upgrade::BakeryExpansion>>();
        query.iter(world).collect::<Vec<_>>()
    };
    for entity in old {
        world.entity_mut(entity).despawn();
    }
    if !enabled {
        return;
    }
    let Some(mut meshes) = world.remove_resource::<Assets<Mesh>>() else {
        return;
    };
    let Some(mut materials) = world.remove_resource::<Assets<StandardMaterial>>() else {
        world.insert_resource(meshes);
        return;
    };
    let wall = materials.add(Color::srgb(0.89, 0.70, 0.43));
    let roof = materials.add(Color::srgb(0.70, 0.27, 0.17));
    {
        let mut commands = world.commands();
        commands.entity(bakery).with_children(|children| {
            children.spawn((
                Name::new("Bakery expanded side room"),
                crate::npc_property_upgrade::BakeryExpansion,
                Mesh3d(meshes.add(Cuboid::new(2.5, 1.8, 1.7))),
                MeshMaterial3d(wall),
                Transform::from_xyz(1.45, 0.9, -0.15),
            ));
            children.spawn((
                Name::new("Bakery expanded roof"),
                crate::npc_property_upgrade::BakeryExpansion,
                Mesh3d(meshes.add(Cuboid::new(2.9, 0.18, 2.5))),
                MeshMaterial3d(roof),
                Transform::from_xyz(0.72, 1.98, -0.02),
            ));
        });
    }
    world.insert_resource(meshes);
    world.insert_resource(materials);
    world.flush();
}

fn restore_carpenter_event_visuals(world: &mut World) {
    let slots = {
        let mut query = world.query::<(Entity, &NpcPropertyEventSlot)>();
        query
            .iter(world)
            .map(|(entity, slot)| (entity, slot.occupied_by))
            .collect::<Vec<_>>()
    };
    let previous = {
        let mut query = world.query::<(Entity, &crate::npc_production::CarpenterBench, &ChildOf)>();
        query
            .iter(world)
            .filter(|(_, _, parent)| slots.iter().any(|(slot, _)| *slot == parent.parent()))
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>()
    };
    for entity in previous {
        world.entity_mut(entity).despawn();
    }
    let occupied = slots
        .into_iter()
        .filter(|(_, event)| event.is_some())
        .map(|(slot, _)| slot)
        .collect::<Vec<_>>();
    if occupied.is_empty() {
        return;
    }
    let Some(mut meshes) = world.remove_resource::<Assets<Mesh>>() else {
        return;
    };
    let Some(mut materials) = world.remove_resource::<Assets<StandardMaterial>>() else {
        world.insert_resource(meshes);
        return;
    };
    let mesh = meshes.add(Cuboid::new(1.4, 0.18, 0.48));
    let material = materials.add(Color::srgb(0.42, 0.25, 0.13));
    {
        let mut commands = world.commands();
        for slot in occupied {
            commands.entity(slot).with_children(|children| {
                children.spawn((
                    Name::new("Carpenter's handmade bench"),
                    crate::npc_production::CarpenterBench,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, 0.25, 0.0),
                ));
            });
        }
    }
    world.insert_resource(meshes);
    world.insert_resource(materials);
    world.flush();
}

fn sync_npc_ceramic_visuals(world: &mut World) {
    let all_slots = {
        let mut q = world.query_filtered::<Entity, With<NpcPlacementSlot>>();
        q.iter(world).collect::<HashSet<_>>()
    };
    let visuals = {
        let mut q = world.query::<(Entity, &CeramicVisual, &ChildOf)>();
        q.iter(world)
            .filter(|(_, _, parent)| all_slots.contains(&parent.parent()))
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>()
    };
    for entity in visuals {
        world.entity_mut(entity).despawn();
    }
    let occupied = {
        let mut q = world.query::<(Entity, &NpcPlacementSlot)>();
        q.iter(world)
            .filter_map(|(entity, slot)| slot.occupied_by.map(|item| (entity, item)))
            .collect::<Vec<_>>()
    };
    for (slot, item) in occupied {
        spawn_visual(world, item, Transform::IDENTITY, Some(slot), None);
    }
}

fn restore_player_placements(world: &mut World, placements: &[PlayerPlacedSave]) {
    let wanted: HashSet<_> = placements.iter().map(|placed| placed.item.id).collect();
    let existing = {
        let mut q = world.query::<(Entity, &PlayerPlacedObject)>();
        q.iter(world)
            .map(|(entity, placed)| (entity, placed.item.id))
            .collect::<Vec<_>>()
    };
    for (entity, id) in &existing {
        if !wanted.contains(id) {
            world.entity_mut(*entity).despawn();
        }
    }
    for saved in placements {
        let direction = Vec3::from_array(saved.direction).normalize();
        let orientation = crate::placement::placement_rotation(direction, saved.rotation)
            .unwrap_or(Quat::IDENTITY);
        let transform = Transform {
            translation: direction * (crate::planet::DEFAULT_PLANET_RADIUS + saved.altitude),
            rotation: orientation,
            ..default()
        };
        let placed = PlayerPlacedObject {
            item: saved.item,
            location: crate::surface_transform::SurfaceLocation::new(direction, saved.altitude),
            rotation: saved.rotation,
        };
        if let Some((entity, _)) = existing.iter().find(|(_, id)| *id == saved.item.id) {
            world.entity_mut(*entity).insert((
                placed,
                transform,
                crate::interaction::Interactable::new("Pick up ceramic"),
            ));
            if let Some(mut visual) = world.get_mut::<CeramicVisual>(*entity) {
                visual.0 = saved.item;
            }
        } else {
            spawn_visual(world, saved.item, transform, None, Some(placed));
        }
    }
}

fn spawn_visual(
    world: &mut World,
    item: CeramicItem,
    transform: Transform,
    parent: Option<Entity>,
    placed: Option<PlayerPlacedObject>,
) {
    let Some(mut meshes) = world.remove_resource::<Assets<Mesh>>() else {
        return;
    };
    let Some(mut materials) = world.remove_resource::<Assets<StandardMaterial>>() else {
        world.insert_resource(meshes);
        return;
    };
    {
        let mut commands = world.commands();
        let entity = crate::ceramic_visuals::spawn_ceramic_visual(
            &mut commands,
            &mut meshes,
            &mut materials,
            item,
            transform,
        );
        if let Some(parent) = parent {
            commands.entity(entity).insert(ChildOf(parent));
        }
        if let Some(placed) = placed {
            commands.entity(entity).insert((
                placed,
                crate::interaction::Interactable::new("Pick up ceramic"),
            ));
        }
    }
    world.insert_resource(meshes);
    world.insert_resource(materials);
    world.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ceramics::{CeramicForm, CeramicItemTemplate, ClayMaterial, Glaze, ProcessingState};

    fn ceramic(id: u64) -> CeramicItem {
        CeramicItemTemplate {
            form: CeramicForm::Cup,
            clay: ClayMaterial::Red,
            glaze: Glaze::Blue,
            state: ProcessingState::Fired,
        }
        .instantiate(CeramicObjectId(id))
    }

    #[test]
    fn malformed_absent_and_incompatible_saves_start_clean() {
        assert!(GameSave::decode("").is_none());
        assert!(GameSave::decode("not json").is_none());
        let mut save = GameSave::clean();
        save.version += 1;
        assert!(GameSave::decode(&save.encode().unwrap()).is_none());
        let mut save = GameSave::clean();
        save.inventory.pop();
        assert!(GameSave::decode(&save.encode().unwrap()).is_none());
    }

    #[test]
    fn file_slot_handles_absent_invalid_and_valid_documents() {
        let path =
            std::env::temp_dir().join(format!("bisque-save-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(read_slot(&path).is_none());
        std::fs::write(&path, "{broken").unwrap();
        assert!(read_slot(&path).is_none());
        let save = GameSave::clean();
        write_slot(&path, &save).unwrap();
        assert_eq!(read_slot(&path), Some(save));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn save_format_round_trips_each_requested_category_and_rejects_bad_identity_data() {
        let mut save = GameSave::clean();
        save.inventory[0] = InventorySlot::ResourceStack {
            resource: ResourceType::RedClay,
            count: 6,
        };
        save.inventory[1] = InventorySlot::Ceramic(CeramicObjectId(7));
        save.coins = 78;
        save.day = 4;
        save.minute_of_day = 731.5;
        save.ceramics = vec![
            ceramic(1),
            ceramic(2),
            ceramic(3),
            ceramic(4),
            ceramic(5),
            ceramic(6),
            ceramic(7),
        ];
        save.next_ceramic_id = 7;
        save.player_direction = Vec3::new(0.2, 0.9, 0.3).normalize().to_array();
        save.player_heading = Vec3::X.to_array();
        save.workbench_upgraded = true;
        save.drying_upgraded = true;
        save.drying_jobs[0] = Some(MachineJobSave {
            object: CeramicObjectId(2),
            started_at: 400.0,
        });
        save.kiln_upgraded = true;
        save.firing_jobs[1] = Some(MachineJobSave {
            object: CeramicObjectId(3),
            started_at: 500.0,
        });
        save.npcs.push(NpcSave {
            character: NpcCharacter::Baker,
            friendship: 56,
            request_state: Some(NpcRequestState::Active),
            baker_final_order_state: Some(NpcRequestState::Complete),
            bakery_upgrade_complete: true,
        });
        save.gathered_resources
            .push(TileCoordinate::new(crate::planet::PlanetFace::PositiveY, 3, 4).unwrap());
        save.player_placements.push(PlayerPlacedSave {
            item: ceramic(4),
            direction: Vec3::Y.to_array(),
            altitude: 0.08,
            rotation: 1.0,
        });
        save.placement_slots.push(PlacementSlotSave {
            property: NpcProperty::Bakery,
            index: 0,
            ceramic: Some(ceramic(5)),
            event: None,
        });
        save.red_clay_discovered = true;
        save.objectives = ObjectiveProgress {
            current: crate::objectives::Objective::UpgradeKiln,
            tracked_cup: Some(CeramicObjectId(7)),
        };
        let encoded = save.encode().unwrap();
        assert_eq!(GameSave::decode(&encoded), Some(save.clone()));
        save.ceramics.push(ceramic(1));
        assert!(!save.validate());
    }

    #[test]
    fn ecs_round_trip_restores_every_gameplay_category_without_duplicate_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>();
        let world = app.world_mut();
        world.init_resource::<Inventory>();
        world.init_resource::<Wallet>();
        world.init_resource::<GameClock>();
        world.init_resource::<CraftedCeramics>();
        world.init_resource::<RedClayDiscovery>();
        world.init_resource::<ObjectiveProgress>();
        world.init_resource::<AppliedNpcPropertyEvents>();
        {
            let mut inventory = world.resource_mut::<Inventory>();
            inventory.add_resource(ResourceType::RedClay, 9);
            assert!(inventory.add_ceramic(CeramicObjectId(1)));
        }
        world.resource_mut::<Wallet>().coins = 92;
        world.resource_mut::<GameClock>().restore(5, 777.25);
        world
            .resource_mut::<CraftedCeramics>()
            .restore((1..=13).map(ceramic).collect(), 13);
        world.resource_mut::<RedClayDiscovery>().discovered = true;
        world.resource_mut::<ObjectiveProgress>().current =
            crate::objectives::Objective::UpgradeKiln;
        world.resource_mut::<ObjectiveProgress>().tracked_cup = Some(CeramicObjectId(7));

        let player = crate::player_movement::SurfacePlayer::new(
            crate::surface_transform::SurfaceLocation::new(
                Vec3::new(0.2, 0.95, 0.1).normalize(),
                1.7,
            ),
            Vec3::X,
            crate::planet::DEFAULT_PLANET_RADIUS,
        );
        world.spawn((player, Transform::default()));
        world.spawn(Workbench { upgraded: true });
        let mut rack = DryingRack::default();
        rack.restore(
            [
                Some(DryingJob::new(CeramicObjectId(2), 600.0)),
                None,
                None,
                None,
            ],
            true,
        );
        world.spawn(rack);
        let mut kiln = Kiln::default();
        kiln.restore(
            [
                None,
                Some(FiringJob::new(CeramicObjectId(3), 700.0)),
                None,
                None,
            ],
            true,
        );
        world.spawn(kiln);

        let baker = world
            .spawn((
                NpcCharacter::Baker,
                NpcFriendship(56),
                crate::npc_requests::NpcRequest {
                    definition: crate::npc_requests::BAKER_TWO_CUPS,
                    state: NpcRequestState::Active,
                },
                BakerFinalOrder {
                    state: NpcRequestState::Complete,
                },
            ))
            .id();
        let bakery = world
            .spawn((NpcProperty::Bakery, BakeryUpgrade { complete: true }))
            .id();
        let bakery_slot = world
            .spawn((NpcPlacementSlot::bakery(0), ChildOf(bakery)))
            .id();
        world
            .entity_mut(bakery_slot)
            .get_mut::<NpcPlacementSlot>()
            .unwrap()
            .occupied_by = Some(ceramic(5));
        world.spawn((NpcPlacementSlot::bakery(1), ChildOf(bakery)));
        for index in 2..9 {
            let mut slot = NpcPlacementSlot::bakery(index);
            slot.occupied_by = Some(ceramic((index + 5) as u64));
            world.spawn((slot, ChildOf(bakery)));
        }
        let workshop = world.spawn((NpcProperty::Workshop,)).id();
        let event_slot = world
            .spawn((NpcPropertyEventSlot::workshop(0), ChildOf(workshop)))
            .id();
        world
            .entity_mut(event_slot)
            .get_mut::<NpcPropertyEventSlot>()
            .unwrap()
            .occupied_by = Some(NpcPropertyEvent::CarpenterAddsBench);
        let coordinate = TileCoordinate::new(crate::planet::PlanetFace::PositiveX, 7, 9).unwrap();
        world.spawn((
            ResourceNode {
                coordinate,
                biome: crate::planet::Biome::RedHighlands,
                resource_type: ResourceType::RedClay,
            },
            GatheredRedClay,
        ));
        let placed = PlayerPlacedObject {
            item: ceramic(4),
            location: crate::surface_transform::SurfaceLocation::new(Vec3::Z, 0.08),
            rotation: 0.5,
        };
        spawn_visual(world, placed.item, Transform::default(), None, Some(placed));
        let saved = capture_save(world);
        let json = saved.encode().unwrap();
        let decoded = GameSave::decode(&json).unwrap();
        assert_eq!(decoded, saved);

        world.resource_mut::<Wallet>().coins = 0;
        world
            .resource_mut::<Inventory>()
            .restore_slots([InventorySlot::Empty; INVENTORY_SLOT_COUNT]);
        world
            .entity_mut(baker)
            .get_mut::<NpcFriendship>()
            .unwrap()
            .0 = 0;
        world
            .entity_mut(bakery)
            .get_mut::<BakeryUpgrade>()
            .unwrap()
            .complete = false;
        world
            .entity_mut(bakery_slot)
            .get_mut::<NpcPlacementSlot>()
            .unwrap()
            .occupied_by = None;
        world
            .entity_mut(event_slot)
            .get_mut::<NpcPropertyEventSlot>()
            .unwrap()
            .occupied_by = None;
        apply_save(world, &decoded);
        assert_eq!(capture_save(world), decoded);
        let mut placed = world.query_filtered::<Entity, With<PlayerPlacedObject>>();
        assert_eq!(placed.iter(world).count(), 1);
        let mut gathered = world.query_filtered::<Entity, With<GatheredRedClay>>();
        assert_eq!(gathered.iter(world).count(), 1);
        let mut npc_visuals = world.query::<(&CeramicVisual, &ChildOf)>();
        assert_eq!(
            npc_visuals
                .iter(world)
                .filter(|(_, parent)| world.get::<NpcPlacementSlot>(parent.parent()).is_some())
                .count(),
            8
        );
        apply_save(world, &decoded);
        let mut placed = world.query_filtered::<Entity, With<PlayerPlacedObject>>();
        assert_eq!(placed.iter(world).count(), 1);
        let mut visuals = world.query::<&CeramicVisual>();
        assert_eq!(
            visuals
                .iter(world)
                .filter(|visual| visual.0.id == CeramicObjectId(4))
                .count(),
            1
        );
    }
}
