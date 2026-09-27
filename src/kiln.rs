//! Two-slot kiln firing jobs for dry ceramics.

use bevy::{input::mouse::MouseButton, prelude::*};

use crate::{
    ceramics::{CeramicItem, ProcessingState},
    economy::Wallet,
    game_clock::GameClock,
    interaction::{Interactable, InteractionRequested},
    inventory::{CeramicObjectId, Inventory},
    machine_upgrades::{MachineUpgradeCost, MachineUpgradeError, pay_machine_upgrade},
    planet::ResourceType,
    planet::{DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, TileCoordinate, sample_tile_surface},
    surface_transform::{SurfaceLocation, surface_transform},
    workbench::CraftedCeramics,
};

/// The base kiln holds two ceramics; firing takes four in-game hours.
pub const KILN_CAPACITY: usize = 2;
pub const KILN_MAX_CAPACITY: usize = 4;
pub const KILN_UPGRADE_COST: MachineUpgradeCost = MachineUpgradeCost {
    coins: 40,
    material: ResourceType::IronMineral,
    quantity: 3,
};
pub const FIRING_TIME_MINUTES: f64 = 4.0 * 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiringJob {
    pub object: CeramicObjectId,
    started_at: f64,
}

impl FiringJob {
    pub(crate) const fn new(object: CeramicObjectId, started_at: f64) -> Self {
        Self { object, started_at }
    }

    pub(crate) const fn started_at(self) -> f64 {
        self.started_at
    }
}

/// Contents of the base kiln. Completed items remain in their slots until collected.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Kiln {
    slots: [Option<FiringJob>; KILN_MAX_CAPACITY],
    pub upgraded: bool,
}

impl Kiln {
    pub fn slots(&self) -> &[Option<FiringJob>; KILN_MAX_CAPACITY] {
        &self.slots
    }

    pub fn occupied(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    pub const fn capacity(&self) -> usize {
        if self.upgraded {
            KILN_MAX_CAPACITY
        } else {
            KILN_CAPACITY
        }
    }

    pub(crate) fn restore(
        &mut self,
        slots: [Option<FiringJob>; KILN_MAX_CAPACITY],
        upgraded: bool,
    ) {
        self.slots = slots;
        self.upgraded = upgraded;
    }
}

pub fn upgrade_kiln(
    kiln: &mut Kiln,
    inventory: &mut Inventory,
    wallet: &mut Wallet,
) -> Result<(), MachineUpgradeError> {
    pay_machine_upgrade(kiln.upgraded, KILN_UPGRADE_COST, inventory, wallet)?;
    kiln.upgraded = true;
    Ok(())
}

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KilnUse(pub Option<Entity>);

#[derive(Resource, Debug, Default)]
struct KilnFeedback(Option<String>);

#[derive(Component)]
struct KilnHelpText;

#[derive(Component)]
struct KilnSlotVisual(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KilnError {
    NotDry,
    NotInInventory,
    Full,
    SlotEmpty,
    NotFired,
    InventoryFull,
}

fn game_minutes(clock: &GameClock) -> f64 {
    (clock.day().saturating_sub(1) as f64 * 24.0 * 60.0) + clock.minute_of_day()
}

/// Move an inventory dry ceramic into the first free slot without consuming it on failure.
pub fn insert_dry_ceramic(
    kiln: &mut Kiln,
    inventory: &mut Inventory,
    crafted: &[CeramicItem],
    object: CeramicObjectId,
    clock: &GameClock,
) -> Result<usize, KilnError> {
    let Some(item) = crafted.iter().find(|item| item.id == object) else {
        return Err(KilnError::NotInInventory);
    };
    if item.state() != ProcessingState::Dry {
        return Err(KilnError::NotDry);
    }
    if !inventory.contains_ceramic(object) {
        return Err(KilnError::NotInInventory);
    }
    let Some(slot) = kiln.slots[..kiln.capacity()]
        .iter()
        .position(Option::is_none)
    else {
        return Err(KilnError::Full);
    };
    assert!(
        inventory.remove_ceramic(object),
        "ownership was checked above"
    );
    kiln.slots[slot] = Some(FiringJob {
        object,
        started_at: game_minutes(clock),
    });
    Ok(slot)
}

/// Collect a finished ceramic. A full inventory leaves the completed job in its slot.
pub fn remove_fired_ceramic(
    kiln: &mut Kiln,
    inventory: &mut Inventory,
    crafted: &mut [CeramicItem],
    slot: usize,
    clock: &GameClock,
) -> Result<CeramicObjectId, KilnError> {
    let job = kiln
        .slots
        .get(slot)
        .copied()
        .flatten()
        .ok_or(KilnError::SlotEmpty)?;
    if game_minutes(clock) - job.started_at < FIRING_TIME_MINUTES
        || !crafted
            .iter()
            .any(|item| item.id == job.object && item.state() == ProcessingState::Fired)
    {
        return Err(KilnError::NotFired);
    }
    if !inventory.add_ceramic(job.object) {
        return Err(KilnError::InventoryFull);
    }
    kiln.slots[slot] = None;
    Ok(job.object)
}

pub struct KilnPlugin;

impl Plugin for KilnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KilnUse>()
            .init_resource::<KilnFeedback>()
            .init_resource::<Wallet>()
            .add_systems(Startup, (spawn_kiln, spawn_help_text))
            .add_systems(
                Update,
                (
                    advance_firing_jobs,
                    begin_kiln_use,
                    control_kiln,
                    refresh_kiln_visuals,
                    update_help_text,
                )
                    .chain(),
            );
    }
}

fn spawn_help_text(mut commands: Commands) {
    commands.spawn((
        KilnHelpText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(112),
            left: px(16),
            ..default()
        },
    ));
}

fn spawn_kiln(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let brick = materials.add(Color::srgb(0.48, 0.23, 0.16));
    let ceramic = materials.add(Color::srgb(0.83, 0.70, 0.55));
    let body = meshes.add(Cuboid::new(1.8, 1.4, 1.4));
    let opening = meshes.add(Cuboid::new(0.9, 0.8, 0.08));
    let pot = meshes.add(Cylinder::new(0.22, 0.34));
    let tile = PlanetTile::new(
        TileCoordinate::new(PlanetFace::PositiveY, 11, 14)
            .expect("kiln starts beside the test player's starting tile"),
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
    .expect("the kiln's tangent orientation is valid");
    transform.translation += transform.rotation * Vec3::X * 5.0;
    let kiln = commands
        .spawn((
            Name::new("Two-slot kiln"),
            Kiln::default(),
            Interactable::new("Use kiln"),
            transform,
            Visibility::default(),
        ))
        .id();
    commands.entity(kiln).with_children(|children| {
        children.spawn((
            Mesh3d(body),
            MeshMaterial3d(brick.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        children.spawn((
            Mesh3d(opening),
            MeshMaterial3d(ceramic.clone()),
            Transform::from_xyz(0.0, -0.1, 0.72),
        ));
        for (slot, x) in [-0.42, -0.14, 0.14, 0.42].into_iter().enumerate() {
            children.spawn((
                Name::new(format!("Kiln slot {slot}")),
                KilnSlotVisual(slot),
                Mesh3d(pot.clone()),
                MeshMaterial3d(ceramic.clone()),
                Transform::from_xyz(x, -0.1, 0.8),
                Visibility::Hidden,
            ));
        }
    });
}

fn complete_firing_jobs(kiln: &Kiln, crafted: &mut [CeramicItem], now: f64) {
    for job in kiln.slots.iter().flatten() {
        if now - job.started_at >= FIRING_TIME_MINUTES
            && let Some(item) = crafted.iter_mut().find(|item| item.id == job.object)
            && item.state() == ProcessingState::Dry
        {
            item.transition_to(ProcessingState::Fired)
                .expect("firing advances dry ceramics to fired");
        }
    }
}

fn advance_firing_jobs(
    kilns: Query<&Kiln>,
    mut crafted: ResMut<CraftedCeramics>,
    clock: Res<GameClock>,
) {
    let Ok(kiln) = kilns.single() else { return };
    complete_firing_jobs(kiln, &mut crafted.items, game_minutes(&clock));
}

fn begin_kiln_use(
    mut requests: MessageReader<InteractionRequested>,
    kilns: Query<(), With<Kiln>>,
    mut active: ResMut<KilnUse>,
    mut feedback: ResMut<KilnFeedback>,
) {
    for request in requests.read() {
        if kilns.contains(request.target) {
            active.0 = Some(request.target);
            feedback.0 = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn control_kiln(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut active: ResMut<KilnUse>,
    mut kilns: Query<&mut Kiln>,
    mut inventory: ResMut<Inventory>,
    mut crafted: ResMut<CraftedCeramics>,
    clock: Res<GameClock>,
    mut wallet: ResMut<Wallet>,
    mut feedback: ResMut<KilnFeedback>,
) {
    let Some(entity) = active.0 else { return };
    if keyboard.just_pressed(KeyCode::Escape)
        || mouse.is_some_and(|input| input.just_pressed(MouseButton::Right))
    {
        active.0 = None;
        feedback.0 = None;
        return;
    }
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }
    let Ok(mut kiln) = kilns.get_mut(entity) else {
        active.0 = None;
        return;
    };
    if keyboard.just_pressed(KeyCode::KeyU) {
        feedback.0 = Some(match upgrade_kiln(&mut kiln, &mut inventory, &mut wallet) {
            Ok(()) => "Kiln upgraded to 4 slots.".to_owned(),
            Err(MachineUpgradeError::AlreadyUpgraded) => "Kiln is already upgraded.".to_owned(),
            Err(MachineUpgradeError::InsufficientCoins) => {
                "Kiln upgrade needs 40 coins.".to_owned()
            }
            Err(MachineUpgradeError::InsufficientMaterials) => {
                "Kiln upgrade needs 3 iron.".to_owned()
            }
        });
        return;
    }
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }
    if let Some((slot, _)) = kiln.slots[..kiln.capacity()]
        .iter()
        .enumerate()
        .find(|(_, job)| {
            job.is_some_and(|job| game_minutes(&clock) - job.started_at >= FIRING_TIME_MINUTES)
        })
    {
        feedback.0 = Some(
            match remove_fired_ceramic(&mut kiln, &mut inventory, &mut crafted.items, slot, &clock)
            {
                Ok(_) => "Fired ceramic collected.".to_owned(),
                Err(KilnError::InventoryFull) => "Inventory is full.".to_owned(),
                _ => "Firing is still in progress.".to_owned(),
            },
        );
        return;
    }
    let Some(object) = crafted
        .items
        .iter()
        .find(|item| inventory.contains_ceramic(item.id) && item.state() == ProcessingState::Dry)
        .map(|item| item.id)
    else {
        feedback.0 = Some("No dry ceramic in inventory.".to_owned());
        return;
    };
    feedback.0 = Some(
        match insert_dry_ceramic(&mut kiln, &mut inventory, &crafted.items, object, &clock) {
            Ok(slot) => format!("Dry ceramic placed in slot {}.", slot + 1),
            Err(KilnError::Full) => "Kiln is full; dry ceramic remains in inventory.".to_owned(),
            Err(_) => "Could not place dry ceramic.".to_owned(),
        },
    );
}

fn refresh_kiln_visuals(
    kilns: Query<&Kiln>,
    mut visuals: Query<(&KilnSlotVisual, &mut Visibility)>,
) {
    let Ok(kiln) = kilns.single() else { return };
    for (slot, mut visibility) in &mut visuals {
        *visibility = if kiln.slots[slot.0].is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn update_help_text(
    active: Res<KilnUse>,
    kilns: Query<&Kiln>,
    feedback: Res<KilnFeedback>,
    mut labels: Query<&mut Text, With<KilnHelpText>>,
) {
    let message = if let Some(entity) = active.0 {
        if let Ok(kiln) = kilns.get(entity) {
            format!(
                "Kiln ({}/{}) — U Upgrade (40 coins + 3 iron) | Enter: fire dry ceramic / collect fired item | Esc: close{}",
                kiln.capacity(),
                kiln.occupied(),
                feedback
                    .0
                    .as_ref()
                    .map_or_else(String::new, |text| format!(" — {text}"))
            )
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

    fn dry_item(id: u64) -> CeramicItem {
        CeramicItemTemplate {
            form: CeramicForm::Vase,
            clay: ClayMaterial::Red,
            glaze: Glaze::Blue,
            state: ProcessingState::Dry,
        }
        .instantiate(CeramicObjectId(id))
    }

    #[test]
    fn kiln_capacity_rejects_extra_dry_ceramic_without_consuming_it() {
        let mut kiln = Kiln::default();
        let mut inventory = Inventory::default();
        let items = [dry_item(1), dry_item(2), dry_item(3)];
        for item in items {
            assert!(inventory.add_ceramic(item.id));
        }
        let clock = GameClock::default();
        assert_eq!(
            insert_dry_ceramic(
                &mut kiln,
                &mut inventory,
                &items,
                CeramicObjectId(1),
                &clock
            ),
            Ok(0)
        );
        assert_eq!(
            insert_dry_ceramic(
                &mut kiln,
                &mut inventory,
                &items,
                CeramicObjectId(2),
                &clock
            ),
            Ok(1)
        );
        let before = inventory.clone();
        assert_eq!(
            insert_dry_ceramic(
                &mut kiln,
                &mut inventory,
                &items,
                CeramicObjectId(3),
                &clock
            ),
            Err(KilnError::Full)
        );
        assert_eq!(inventory, before);
        assert!(inventory.contains_ceramic(CeramicObjectId(3)));
    }

    #[test]
    fn only_dry_owned_items_can_be_inserted() {
        let mut kiln = Kiln::default();
        let mut inventory = Inventory::default();
        let greenware = CeramicItemTemplate {
            state: ProcessingState::Greenware,
            ..dry_item(1).template
        }
        .instantiate(CeramicObjectId(1));
        let fired = CeramicItemTemplate {
            state: ProcessingState::Fired,
            ..dry_item(2).template
        }
        .instantiate(CeramicObjectId(2));
        inventory.add_ceramic(greenware.id);
        inventory.add_ceramic(fired.id);
        for item in [greenware, fired] {
            assert_eq!(
                insert_dry_ceramic(
                    &mut kiln,
                    &mut inventory,
                    &[greenware, fired],
                    item.id,
                    &GameClock::default()
                ),
                Err(KilnError::NotDry)
            );
            assert!(inventory.contains_ceramic(item.id));
        }
        assert_eq!(
            insert_dry_ceramic(
                &mut kiln,
                &mut inventory,
                &[dry_item(3)],
                CeramicObjectId(3),
                &GameClock::default()
            ),
            Err(KilnError::NotInInventory)
        );
    }

    #[test]
    fn dry_item_remains_dry_until_exact_four_hour_boundary_then_fires_with_identity_and_glaze() {
        let mut kiln = Kiln::default();
        let mut inventory = Inventory::default();
        let mut items = vec![dry_item(5)];
        inventory.add_ceramic(CeramicObjectId(5));
        let mut clock = GameClock::default();
        insert_dry_ceramic(
            &mut kiln,
            &mut inventory,
            &items,
            CeramicObjectId(5),
            &clock,
        )
        .unwrap();
        clock.advance_real_seconds((FIRING_TIME_MINUTES - 0.01) / 6.0);
        complete_firing_jobs(&kiln, &mut items, game_minutes(&clock));
        assert_eq!(items[0].state(), ProcessingState::Dry);
        assert_eq!(
            remove_fired_ceramic(&mut kiln, &mut inventory, &mut items, 0, &clock),
            Err(KilnError::NotFired)
        );
        clock.advance_real_seconds(0.01 / 6.0);
        complete_firing_jobs(&kiln, &mut items, game_minutes(&clock));
        assert_eq!(items[0].state(), ProcessingState::Fired);
        assert_eq!(items[0].id, CeramicObjectId(5));
        assert_eq!(items[0].form(), CeramicForm::Vase);
        assert_eq!(items[0].clay(), ClayMaterial::Red);
        assert_eq!(items[0].glaze(), Glaze::Blue);
        assert_eq!(
            remove_fired_ceramic(&mut kiln, &mut inventory, &mut items, 0, &clock),
            Ok(CeramicObjectId(5))
        );
        assert!(inventory.contains_ceramic(CeramicObjectId(5)));
    }

    #[test]
    fn scheduled_kiln_job_transitions_at_the_four_hour_boundary() {
        let mut kiln = Kiln::default();
        let mut inventory = Inventory::default();
        let item = dry_item(9);
        inventory.add_ceramic(item.id);
        let clock = GameClock::default();
        insert_dry_ceramic(&mut kiln, &mut inventory, &[item], item.id, &clock).unwrap();
        let mut app = App::new();
        app.insert_resource(clock)
            .init_resource::<CraftedCeramics>()
            .add_systems(Update, advance_firing_jobs);
        app.world_mut()
            .resource_mut::<CraftedCeramics>()
            .items
            .push(item);
        app.world_mut().spawn(kiln);
        app.world_mut()
            .resource_mut::<GameClock>()
            .advance_real_seconds((FIRING_TIME_MINUTES - 0.01) / 6.0);
        app.update();
        assert_eq!(
            app.world().resource::<CraftedCeramics>().items[0].state(),
            ProcessingState::Dry
        );
        app.world_mut()
            .resource_mut::<GameClock>()
            .advance_real_seconds(0.01 / 6.0);
        app.update();
        assert_eq!(
            app.world().resource::<CraftedCeramics>().items[0].state(),
            ProcessingState::Fired
        );
    }

    #[test]
    fn kiln_upgrade_expands_capacity_once_and_charges_coins_and_iron() {
        let mut kiln = Kiln::default();
        let mut inventory = Inventory::default();
        let mut wallet = Wallet {
            coins: KILN_UPGRADE_COST.coins,
        };
        inventory.add_resource(KILN_UPGRADE_COST.material, KILN_UPGRADE_COST.quantity);
        assert_eq!(kiln.capacity(), 2);
        upgrade_kiln(&mut kiln, &mut inventory, &mut wallet).unwrap();
        assert_eq!(kiln.capacity(), 4);
        assert_eq!(wallet.coins, 0);
        assert_eq!(inventory.resource_count(KILN_UPGRADE_COST.material), 0);
        assert_eq!(
            upgrade_kiln(&mut kiln, &mut inventory, &mut wallet),
            Err(MachineUpgradeError::AlreadyUpgraded)
        );
        assert_eq!(kiln.capacity(), 4);
        assert_eq!(wallet.coins, 0);
    }

    #[test]
    fn kiln_upgrade_insufficient_coins_or_materials_is_atomic() {
        let mut kiln = Kiln::default();
        let mut inventory = Inventory::default();
        let mut wallet = Wallet {
            coins: KILN_UPGRADE_COST.coins - 1,
        };
        inventory.add_resource(KILN_UPGRADE_COST.material, KILN_UPGRADE_COST.quantity);
        let before = inventory.clone();
        assert_eq!(
            upgrade_kiln(&mut kiln, &mut inventory, &mut wallet),
            Err(MachineUpgradeError::InsufficientCoins)
        );
        assert_eq!(wallet.coins, KILN_UPGRADE_COST.coins - 1);
        assert_eq!(inventory, before);
        wallet.coins += 1;
        inventory.remove_resource(KILN_UPGRADE_COST.material, 1);
        let before = inventory.clone();
        assert_eq!(
            upgrade_kiln(&mut kiln, &mut inventory, &mut wallet),
            Err(MachineUpgradeError::InsufficientMaterials)
        );
        assert_eq!(wallet.coins, KILN_UPGRADE_COST.coins);
        assert_eq!(inventory, before);
        assert_eq!(kiln.capacity(), 2);
    }
}
