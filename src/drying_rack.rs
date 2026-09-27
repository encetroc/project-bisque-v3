//! Two-slot drying rack jobs for greenware.

use bevy::{input::mouse::MouseButton, prelude::*};

use crate::{
    ceramics::{CeramicItem, ProcessingState},
    economy::Wallet,
    game_clock::GameClock,
    interaction::{Interactable, InteractionRequested},
    inventory::{CeramicObjectId, Inventory},
    machine_upgrades::{MachineUpgradeCost, MachineUpgradeError, pay_machine_upgrade},
    planet::{DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TileCoordinate, sample_tile_surface},
    surface_transform::{SurfaceLocation, surface_transform},
    workbench::CraftedCeramics,
};

/// The base rack holds two ceramics; drying takes four in-game hours.
pub const DRYING_RACK_CAPACITY: usize = 2;
pub const DRYING_RACK_MAX_CAPACITY: usize = 4;
pub const DRYING_RACK_UPGRADE_COST: MachineUpgradeCost = MachineUpgradeCost {
    coins: 20,
    material: crate::planet::ResourceType::Wood,
    quantity: 5,
};
pub const DRYING_TIME_MINUTES: f64 = 4.0 * 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DryingJob {
    pub object: CeramicObjectId,
    started_at: f64,
}

impl DryingJob {
    pub(crate) const fn new(object: CeramicObjectId, started_at: f64) -> Self {
        Self { object, started_at }
    }

    pub(crate) const fn started_at(self) -> f64 {
        self.started_at
    }
}

/// Contents of the base drying rack. Completed items remain in their slots until collected.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct DryingRack {
    slots: [Option<DryingJob>; DRYING_RACK_MAX_CAPACITY],
    pub upgraded: bool,
}

impl DryingRack {
    pub fn slots(&self) -> &[Option<DryingJob>; DRYING_RACK_MAX_CAPACITY] {
        &self.slots
    }

    pub fn occupied(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    pub const fn capacity(&self) -> usize {
        if self.upgraded {
            DRYING_RACK_MAX_CAPACITY
        } else {
            DRYING_RACK_CAPACITY
        }
    }

    pub(crate) fn restore(
        &mut self,
        slots: [Option<DryingJob>; DRYING_RACK_MAX_CAPACITY],
        upgraded: bool,
    ) {
        self.slots = slots;
        self.upgraded = upgraded;
    }
}

pub fn upgrade_drying_rack(
    rack: &mut DryingRack,
    inventory: &mut Inventory,
    wallet: &mut Wallet,
) -> Result<(), MachineUpgradeError> {
    pay_machine_upgrade(rack.upgraded, DRYING_RACK_UPGRADE_COST, inventory, wallet)?;
    rack.upgraded = true;
    Ok(())
}

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DryingRackUse(pub Option<Entity>);

#[derive(Resource, Debug, Default)]
struct RackFeedback(Option<String>);

#[derive(Component)]
struct RackHelpText;

#[derive(Component)]
struct RackSlotVisual(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RackError {
    NotGreenware,
    NotInInventory,
    Full,
    SlotEmpty,
    NotDry,
    InventoryFull,
}

fn game_minutes(clock: &GameClock) -> f64 {
    (clock.day().saturating_sub(1) as f64 * 24.0 * 60.0) + clock.minute_of_day()
}

/// Move an inventory greenware object into the first free slot without consuming it on failure.
pub fn insert_greenware(
    rack: &mut DryingRack,
    inventory: &mut Inventory,
    crafted: &[CeramicItem],
    object: CeramicObjectId,
    clock: &GameClock,
) -> Result<usize, RackError> {
    let Some(item) = crafted.iter().find(|item| item.id == object) else {
        return Err(RackError::NotInInventory);
    };
    if item.state() != ProcessingState::Greenware {
        return Err(RackError::NotGreenware);
    }
    if !inventory.contains_ceramic(object) {
        return Err(RackError::NotInInventory);
    }
    let Some(slot) = rack.slots[..rack.capacity()]
        .iter()
        .position(Option::is_none)
    else {
        return Err(RackError::Full);
    };
    assert!(
        inventory.remove_ceramic(object),
        "ownership was checked above"
    );
    rack.slots[slot] = Some(DryingJob {
        object,
        started_at: game_minutes(clock),
    });
    Ok(slot)
}

/// Collect a finished ceramic. A full inventory leaves the completed job in its slot.
pub fn remove_dry_ceramic(
    rack: &mut DryingRack,
    inventory: &mut Inventory,
    crafted: &mut [CeramicItem],
    slot: usize,
    clock: &GameClock,
) -> Result<CeramicObjectId, RackError> {
    let job = rack
        .slots
        .get(slot)
        .copied()
        .flatten()
        .ok_or(RackError::SlotEmpty)?;
    if game_minutes(clock) - job.started_at < DRYING_TIME_MINUTES
        || !crafted
            .iter()
            .any(|item| item.id == job.object && item.state() == ProcessingState::Dry)
    {
        return Err(RackError::NotDry);
    }
    if !inventory.add_ceramic(job.object) {
        return Err(RackError::InventoryFull);
    }
    rack.slots[slot] = None;
    Ok(job.object)
}

pub struct DryingRackPlugin;

impl Plugin for DryingRackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DryingRackUse>()
            .init_resource::<RackFeedback>()
            .init_resource::<Wallet>()
            .init_resource::<Wallet>()
            .add_systems(Startup, (spawn_rack, spawn_help_text))
            .add_systems(
                Update,
                (
                    advance_drying_jobs,
                    begin_rack_use,
                    control_rack,
                    refresh_rack_visuals,
                    update_help_text,
                )
                    .chain(),
            );
    }
}

fn spawn_help_text(mut commands: Commands) {
    commands.spawn((
        RackHelpText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(80),
            left: px(16),
            ..default()
        },
    ));
}

fn spawn_rack(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wood = materials.add(Color::srgb(0.38, 0.22, 0.12));
    let ceramic = materials.add(Color::srgb(0.83, 0.70, 0.55));
    let plank = meshes.add(Cuboid::new(2.2, 0.12, 0.16));
    let post = meshes.add(Cuboid::new(0.14, 1.5, 0.14));
    let pot = meshes.add(Cylinder::new(0.22, 0.34));
    let tile = PlanetTile::new(
        TileCoordinate::new(PlanetFace::PositiveY, 11, 14)
            .expect("rack starts beside the test player's starting tile"),
    );
    let surface = sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
        .expect("the configured planet radius is valid");
    let mut transform = surface_transform(
        SurfaceLocation::new(surface.normal, 0.1),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &surface,
        Vec3::X,
    )
    .expect("the rack's tangent orientation is valid");
    transform.translation += transform.rotation * Vec3::X * 2.5;
    let rack = commands
        .spawn((
            Name::new("Two-slot drying rack"),
            DryingRack::default(),
            Interactable::new("Use drying rack"),
            transform,
            Visibility::default(),
        ))
        .id();
    commands.entity(rack).with_children(|children| {
        for x in [-1.0, 1.0] {
            children.spawn((
                Mesh3d(post.clone()),
                MeshMaterial3d(wood.clone()),
                Transform::from_xyz(x, 0.0, 0.0),
            ));
        }
        for y in [-0.35, 0.1, 0.55] {
            children.spawn((
                Mesh3d(plank.clone()),
                MeshMaterial3d(wood.clone()),
                Transform::from_xyz(0.0, y, 0.0),
            ));
        }
        for (slot, x) in [-0.75, -0.25, 0.25, 0.75].into_iter().enumerate() {
            children.spawn((
                Name::new(format!("Drying rack slot {slot}")),
                RackSlotVisual(slot),
                Mesh3d(pot.clone()),
                MeshMaterial3d(ceramic.clone()),
                Transform::from_xyz(x, 0.32, 0.1),
                Visibility::Hidden,
            ));
        }
    });
}

fn complete_drying_jobs(rack: &DryingRack, crafted: &mut [CeramicItem], now: f64) {
    for job in rack.slots.iter().flatten() {
        if now - job.started_at >= DRYING_TIME_MINUTES
            && let Some(item) = crafted.iter_mut().find(|item| item.id == job.object)
            && item.state() == ProcessingState::Greenware
        {
            item.transition_to(ProcessingState::Dry)
                .expect("drying advances greenware to dry");
        }
    }
}

fn advance_drying_jobs(
    racks: Query<&DryingRack>,
    mut crafted: ResMut<CraftedCeramics>,
    clock: Res<GameClock>,
) {
    let Ok(rack) = racks.single() else { return };
    complete_drying_jobs(rack, &mut crafted.items, game_minutes(&clock));
}

fn begin_rack_use(
    mut requests: MessageReader<InteractionRequested>,
    racks: Query<(), With<DryingRack>>,
    mut active: ResMut<DryingRackUse>,
    mut feedback: ResMut<RackFeedback>,
) {
    for request in requests.read() {
        if racks.contains(request.target) {
            active.0 = Some(request.target);
            feedback.0 = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn control_rack(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut active: ResMut<DryingRackUse>,
    mut racks: Query<&mut DryingRack>,
    mut inventory: ResMut<Inventory>,
    mut crafted: ResMut<CraftedCeramics>,
    clock: Res<GameClock>,
    mut wallet: ResMut<Wallet>,
    mut feedback: ResMut<RackFeedback>,
) {
    let Some(entity) = active.0 else { return };
    if keyboard.just_pressed(KeyCode::Escape)
        || mouse.is_some_and(|input| input.just_pressed(MouseButton::Right))
    {
        active.0 = None;
        feedback.0 = None;
        return;
    }
    let Ok(mut rack) = racks.get_mut(entity) else {
        active.0 = None;
        return;
    };
    if keyboard.just_pressed(KeyCode::KeyU) {
        feedback.0 = Some(
            match upgrade_drying_rack(&mut rack, &mut inventory, &mut wallet) {
                Ok(()) => "Drying rack upgraded to 4 slots.".to_owned(),
                Err(MachineUpgradeError::AlreadyUpgraded) => {
                    "Drying rack is already upgraded.".to_owned()
                }
                Err(MachineUpgradeError::InsufficientCoins) => {
                    "Rack upgrade needs 20 coins.".to_owned()
                }
                Err(MachineUpgradeError::InsufficientMaterials) => {
                    "Rack upgrade needs 5 wood.".to_owned()
                }
            },
        );
        return;
    }
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }
    if let Some(job) = rack.slots[..rack.capacity()]
        .iter()
        .flatten()
        .find(|job| game_minutes(&clock) - job.started_at >= DRYING_TIME_MINUTES)
    {
        let slot = rack.slots[..rack.capacity()]
            .iter()
            .position(|candidate| *candidate == Some(*job))
            .unwrap();
        feedback.0 = Some(
            match remove_dry_ceramic(&mut rack, &mut inventory, &mut crafted.items, slot, &clock) {
                Ok(_) => "Dry ceramic collected.".to_owned(),
                Err(RackError::InventoryFull) => "Inventory is full.".to_owned(),
                _ => unreachable!(),
            },
        );
        return;
    }
    let Some(object) = crafted
        .items
        .iter()
        .find(|item| {
            inventory.contains_ceramic(item.id) && item.state() == ProcessingState::Greenware
        })
        .map(|item| item.id)
    else {
        feedback.0 = Some("No greenware in inventory.".to_owned());
        return;
    };
    feedback.0 = Some(
        match insert_greenware(&mut rack, &mut inventory, &crafted.items, object, &clock) {
            Ok(slot) => format!("Greenware placed in slot {}.", slot + 1),
            Err(RackError::Full) => "Rack is full; greenware remains in inventory.".to_owned(),
            Err(_) => "Could not place greenware.".to_owned(),
        },
    );
}

fn refresh_rack_visuals(
    racks: Query<&DryingRack>,
    mut visuals: Query<(&RackSlotVisual, &mut Visibility)>,
) {
    let Ok(rack) = racks.single() else { return };
    for (slot, mut visibility) in &mut visuals {
        *visibility = if rack.slots[slot.0].is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn update_help_text(
    active: Res<DryingRackUse>,
    racks: Query<&DryingRack>,
    crafted: Res<CraftedCeramics>,
    clock: Res<GameClock>,
    feedback: Res<RackFeedback>,
    mut labels: Query<&mut Text, With<RackHelpText>>,
) {
    let message = if let Some(entity) = active.0 {
        if let Ok(rack) = racks.get(entity) {
            let now = game_minutes(&clock);
            let mut status = format!(
                "Drying rack ({}/{}) — U Upgrade (20 coins + 5 wood) | Enter: place greenware / collect dry item | Esc: close",
                rack.occupied(),
                rack.capacity()
            );
            for (slot, job) in rack.slots[..rack.capacity()].iter().enumerate() {
                let details = job.map_or_else(
                    || "Empty".to_owned(),
                    |job| {
                        let item = crafted.items.iter().find(|item| item.id == job.object);
                        let remaining = (DRYING_TIME_MINUTES - (now - job.started_at)).max(0.0);
                        let progress = item.map_or_else(
                            || "unknown ceramic".to_owned(),
                            |item| {
                                format!(
                                    "{:?} #{:03} ({:?} clay, {:?} glaze, {:?})",
                                    item.form(),
                                    item.id.0,
                                    item.clay(),
                                    item.glaze(),
                                    item.state()
                                )
                            },
                        );
                        format!(
                            "{progress}, {}h {:02}m remaining",
                            (remaining / 60.0) as u64,
                            remaining as u64 % 60
                        )
                    },
                );
                status.push_str(&format!("\nSlot {}: {details}", slot + 1));
            }
            if let Some(feedback) = &feedback.0 {
                status.push_str(&format!("\n{feedback}"));
            }
            status
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    for mut label in &mut labels {
        label.0.clone_from(&message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ceramics::{CeramicForm, CeramicItemTemplate, ClayMaterial, Glaze};

    fn greenware(id: u64) -> CeramicItem {
        CeramicItemTemplate {
            form: CeramicForm::Cup,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
            state: ProcessingState::Greenware,
        }
        .instantiate(CeramicObjectId(id))
    }

    #[test]
    fn rack_panel_displays_the_occupied_item_and_game_time_remaining_headlessly() {
        let item = greenware(41);
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        inventory.add_ceramic(item.id);
        let clock = GameClock::default();
        assert_eq!(
            insert_greenware(&mut rack, &mut inventory, &[item], item.id, &clock),
            Ok(0)
        );

        let mut app = App::new();
        app.insert_resource(clock)
            .insert_resource(DryingRackUse::default())
            .insert_resource(RackFeedback::default())
            .insert_resource(inventory)
            .init_resource::<CraftedCeramics>()
            .add_systems(Update, update_help_text);
        app.world_mut()
            .resource_mut::<CraftedCeramics>()
            .items
            .push(item);
        let rack_entity = app.world_mut().spawn(rack).id();
        app.world_mut().resource_mut::<DryingRackUse>().0 = Some(rack_entity);
        app.world_mut().spawn((RackHelpText, Text::new("")));
        app.update();

        let Ok(text) = app
            .world_mut()
            .query_filtered::<&Text, With<RackHelpText>>()
            .single(app.world())
        else {
            panic!("rack status panel should exist");
        };
        assert!(text.0.contains("Drying rack (1/2)"));
        assert!(text.0.contains("Slot 1: Cup #041"));
        assert!(text.0.contains("Greenware"));
        assert!(text.0.contains("4h 00m remaining"));
        assert!(text.0.contains("Slot 2: Empty"));
    }

    #[test]
    fn rack_capacity_rejects_extra_greenware_without_consuming_it() {
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        let objects = [greenware(1), greenware(2), greenware(3)];
        for item in objects {
            assert!(inventory.add_ceramic(item.id));
        }
        let clock = GameClock::default();
        assert_eq!(
            insert_greenware(
                &mut rack,
                &mut inventory,
                &objects,
                CeramicObjectId(1),
                &clock
            ),
            Ok(0)
        );
        assert_eq!(
            insert_greenware(
                &mut rack,
                &mut inventory,
                &objects,
                CeramicObjectId(2),
                &clock
            ),
            Ok(1)
        );
        let before = inventory.clone();
        assert_eq!(
            insert_greenware(
                &mut rack,
                &mut inventory,
                &objects,
                CeramicObjectId(3),
                &clock
            ),
            Err(RackError::Full)
        );
        assert_eq!(inventory, before);
        assert!(inventory.contains_ceramic(CeramicObjectId(3)));
    }

    #[test]
    fn greenware_remains_until_four_game_hours_then_becomes_dry() {
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        let mut objects = vec![greenware(5)];
        inventory.add_ceramic(CeramicObjectId(5));
        let mut clock = GameClock::default();
        insert_greenware(
            &mut rack,
            &mut inventory,
            &objects,
            CeramicObjectId(5),
            &clock,
        )
        .unwrap();

        clock.advance_real_seconds((DRYING_TIME_MINUTES - 0.01) / 6.0);
        complete_drying_jobs(&rack, &mut objects, game_minutes(&clock));
        assert_eq!(objects[0].state(), ProcessingState::Greenware);
        assert_eq!(
            remove_dry_ceramic(&mut rack, &mut inventory, &mut objects, 0, &clock),
            Err(RackError::NotDry)
        );

        clock.advance_real_seconds(0.01 / 6.0);
        complete_drying_jobs(&rack, &mut objects, game_minutes(&clock));
        assert_eq!(
            game_minutes(&clock) - rack.slots()[0].unwrap().started_at,
            DRYING_TIME_MINUTES
        );
        assert_eq!(objects[0].state(), ProcessingState::Dry);
        assert_eq!(
            remove_dry_ceramic(&mut rack, &mut inventory, &mut objects, 0, &clock),
            Ok(CeramicObjectId(5))
        );
        assert!(inventory.contains_ceramic(CeramicObjectId(5)));
    }

    #[test]
    fn scheduled_rack_job_transitions_at_the_four_hour_boundary() {
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        let item = greenware(9);
        inventory.add_ceramic(item.id);
        let clock = GameClock::default();
        insert_greenware(&mut rack, &mut inventory, &[item], item.id, &clock).unwrap();

        let mut app = App::new();
        app.insert_resource(clock)
            .init_resource::<CraftedCeramics>()
            .add_systems(Update, advance_drying_jobs);
        app.world_mut()
            .resource_mut::<CraftedCeramics>()
            .items
            .push(item);
        app.world_mut().spawn(rack);

        app.world_mut()
            .resource_mut::<GameClock>()
            .advance_real_seconds((DRYING_TIME_MINUTES - 0.01) / 6.0);
        app.update();
        assert_eq!(
            app.world().resource::<CraftedCeramics>().items[0].state(),
            ProcessingState::Greenware
        );

        app.world_mut()
            .resource_mut::<GameClock>()
            .advance_real_seconds(0.01 / 6.0);
        app.update();
        assert_eq!(
            app.world().resource::<CraftedCeramics>().items[0].state(),
            ProcessingState::Dry
        );
    }

    #[test]
    fn only_greenware_can_be_inserted_and_full_inventory_preserves_dry_output() {
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        let mut objects = vec![greenware(7)];
        objects[0].transition_to(ProcessingState::Dry).unwrap();
        inventory.add_ceramic(CeramicObjectId(7));
        assert_eq!(
            insert_greenware(
                &mut rack,
                &mut inventory,
                &objects,
                CeramicObjectId(7),
                &GameClock::default()
            ),
            Err(RackError::NotGreenware)
        );

        objects[0] = greenware(7);
        insert_greenware(
            &mut rack,
            &mut inventory,
            &objects,
            CeramicObjectId(7),
            &GameClock::default(),
        )
        .unwrap();
        let mut clock = GameClock::default();
        clock.advance_real_seconds(DRYING_TIME_MINUTES / 6.0);
        for id in 100..120 {
            inventory.add_ceramic(CeramicObjectId(id));
        }
        complete_drying_jobs(&rack, &mut objects, game_minutes(&clock));
        assert_eq!(
            remove_dry_ceramic(&mut rack, &mut inventory, &mut objects, 0, &clock),
            Err(RackError::InventoryFull)
        );
        assert_eq!(rack.occupied(), 1);
        assert_eq!(objects[0].state(), ProcessingState::Dry);
    }

    #[test]
    fn rack_upgrade_expands_capacity_once_and_charges_coins_and_wood() {
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        let mut wallet = Wallet {
            coins: DRYING_RACK_UPGRADE_COST.coins,
        };
        inventory.add_resource(
            DRYING_RACK_UPGRADE_COST.material,
            DRYING_RACK_UPGRADE_COST.quantity,
        );
        assert_eq!(rack.capacity(), 2);
        upgrade_drying_rack(&mut rack, &mut inventory, &mut wallet).unwrap();
        assert_eq!(rack.capacity(), 4);
        assert_eq!(wallet.coins, 0);
        assert_eq!(
            inventory.resource_count(DRYING_RACK_UPGRADE_COST.material),
            0
        );
        assert_eq!(
            upgrade_drying_rack(&mut rack, &mut inventory, &mut wallet),
            Err(MachineUpgradeError::AlreadyUpgraded)
        );
        assert_eq!(rack.capacity(), 4);
        assert_eq!(wallet.coins, 0);
    }

    #[test]
    fn rack_upgrade_insufficient_materials_preserve_all_payment_and_capacity() {
        let mut rack = DryingRack::default();
        let mut inventory = Inventory::default();
        let mut wallet = Wallet {
            coins: DRYING_RACK_UPGRADE_COST.coins,
        };
        inventory.add_resource(
            DRYING_RACK_UPGRADE_COST.material,
            DRYING_RACK_UPGRADE_COST.quantity - 1,
        );
        let before = inventory.clone();
        assert_eq!(
            upgrade_drying_rack(&mut rack, &mut inventory, &mut wallet),
            Err(MachineUpgradeError::InsufficientMaterials)
        );
        assert_eq!(wallet.coins, DRYING_RACK_UPGRADE_COST.coins);
        assert_eq!(inventory, before);
        assert_eq!(rack.capacity(), 2);
    }
}
