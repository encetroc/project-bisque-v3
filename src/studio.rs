//! Open-air studio layout and its one-time usable expansion.

use bevy::prelude::*;

use crate::{
    economy::Wallet,
    interaction::{Interactable, InteractionRequested},
    inventory::{CeramicObjectId, Inventory},
    machine_upgrades::{MachineUpgradeCost, MachineUpgradeError, pay_machine_upgrade},
    planet::{
        DEFAULT_PLANET_RADIUS, PlanetFace, PlanetTile, ResourceType, TileCoordinate,
        sample_tile_surface,
    },
    surface_transform::{SurfaceLocation, surface_transform},
};

pub const STUDIO_UPGRADE_COST: MachineUpgradeCost = MachineUpgradeCost {
    coins: 50,
    material: ResourceType::Wood,
    quantity: 10,
};
pub const STARTER_STORAGE_CAPACITY: usize = 10;
pub const EXPANDED_STORAGE_CAPACITY: usize = 20;
pub const STARTER_MACHINE_SPACES: usize = 3;
pub const EXPANDED_MACHINE_SPACES: usize = 4;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StudioUpgrade {
    pub upgraded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredItem {
    Resource(ResourceType),
    Ceramic(CeramicObjectId),
}

/// A small studio chest. Its slot count grows with the studio expansion; resources
/// of the same type share one slot, while each ceramic uses one slot.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct StudioStorage {
    capacity: usize,
    items: Vec<StoredItem>,
    resource_counts: Vec<(ResourceType, u32)>,
}

impl Default for StudioStorage {
    fn default() -> Self {
        Self {
            capacity: STARTER_STORAGE_CAPACITY,
            items: Vec::new(),
            resource_counts: Vec::new(),
        }
    }
}

impl StudioUpgrade {
    pub const fn machine_spaces(self) -> usize {
        if self.upgraded {
            EXPANDED_MACHINE_SPACES
        } else {
            STARTER_MACHINE_SPACES
        }
    }
}

impl StudioStorage {
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn used_slots(&self) -> usize {
        self.items.len()
    }

    pub fn contains(&self, item: StoredItem) -> bool {
        self.items.contains(&item)
    }

    pub fn deposit_resource(&mut self, inventory: &mut Inventory, resource: ResourceType) -> bool {
        if inventory.resource_count(resource) == 0 {
            return false;
        }
        let stored_already = self.items.contains(&StoredItem::Resource(resource));
        if !stored_already && self.items.len() >= self.capacity {
            return false;
        }
        if inventory.remove_resource(resource, 1) != 1 {
            return false;
        }
        if !stored_already {
            self.items.push(StoredItem::Resource(resource));
            self.resource_counts.push((resource, 0));
        }
        let (_, count) = self
            .resource_counts
            .iter_mut()
            .find(|(stored_resource, _)| *stored_resource == resource)
            .expect("the resource slot was created above");
        *count += 1;
        true
    }

    pub fn withdraw_resource(&mut self, inventory: &mut Inventory, resource: ResourceType) -> bool {
        let stored = StoredItem::Resource(resource);
        let Some(index) = self.items.iter().position(|item| *item == stored) else {
            return false;
        };
        if inventory.add_resource(resource, 1) != 0 {
            return false;
        }
        let count_index = self
            .resource_counts
            .iter()
            .position(|(stored_resource, _)| *stored_resource == resource)
            .expect("stored resource has a matching quantity");
        let (_, count) = &mut self.resource_counts[count_index];
        *count -= 1;
        if *count == 0 {
            self.resource_counts.remove(count_index);
            self.items.remove(index);
        }
        true
    }

    pub fn deposit_ceramic(&mut self, inventory: &mut Inventory, ceramic: CeramicObjectId) -> bool {
        let stored = StoredItem::Ceramic(ceramic);
        if !inventory.contains_ceramic(ceramic)
            || self.items.contains(&stored)
            || self.items.len() >= self.capacity
        {
            return false;
        }
        if !inventory.remove_ceramic(ceramic) {
            return false;
        }
        self.items.push(stored);
        true
    }

    pub fn withdraw_ceramic(
        &mut self,
        inventory: &mut Inventory,
        ceramic: CeramicObjectId,
    ) -> bool {
        let stored = StoredItem::Ceramic(ceramic);
        let Some(index) = self.items.iter().position(|item| *item == stored) else {
            return false;
        };
        if !inventory.add_ceramic(ceramic) {
            return false;
        }
        self.items.remove(index);
        true
    }

    fn expand(&mut self) {
        self.capacity = EXPANDED_STORAGE_CAPACITY;
    }
}

pub fn upgrade_studio(
    upgrade: &mut StudioUpgrade,
    storage: &mut StudioStorage,
    display: &mut StudioDisplay,
    inventory: &mut Inventory,
    wallet: &mut Wallet,
) -> Result<(), MachineUpgradeError> {
    pay_machine_upgrade(upgrade.upgraded, STUDIO_UPGRADE_COST, inventory, wallet)?;
    upgrade.upgraded = true;
    storage.expand();
    display.expand();
    Ok(())
}

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct StudioDisplay {
    capacity: usize,
    ceramics: Vec<CeramicObjectId>,
}

impl StudioDisplay {
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn ceramics(&self) -> &[CeramicObjectId] {
        &self.ceramics
    }

    pub fn display(&mut self, inventory: &mut Inventory, ceramic: CeramicObjectId) -> bool {
        if self.ceramics.len() >= self.capacity
            || self.ceramics.contains(&ceramic)
            || !inventory.remove_ceramic(ceramic)
        {
            return false;
        }
        self.ceramics.push(ceramic);
        true
    }

    pub fn collect(&mut self, inventory: &mut Inventory, ceramic: CeramicObjectId) -> bool {
        let Some(index) = self.ceramics.iter().position(|item| *item == ceramic) else {
            return false;
        };
        if !inventory.add_ceramic(ceramic) {
            return false;
        }
        self.ceramics.remove(index);
        true
    }

    fn expand(&mut self) {
        self.capacity = 4;
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct StudioExpansion;

pub struct StudioPlugin;

impl Plugin for StudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Wallet>()
            .init_resource::<Inventory>()
            .init_resource::<StudioStorage>()
            .init_resource::<StudioDisplay>()
            .add_systems(Startup, spawn_studio)
            .add_systems(Update, handle_studio_upgrade);
    }
}

fn handle_studio_upgrade(
    mut commands: Commands,
    mut requests: MessageReader<InteractionRequested>,
    mut studios: Query<&mut StudioUpgrade>,
    mut storage: ResMut<StudioStorage>,
    mut display: ResMut<StudioDisplay>,
    mut inventory: ResMut<Inventory>,
    mut wallet: ResMut<Wallet>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for request in requests.read() {
        let Ok(mut upgrade) = studios.get_mut(request.target) else {
            continue;
        };
        if upgrade_studio(
            &mut upgrade,
            &mut storage,
            &mut display,
            &mut inventory,
            &mut wallet,
        )
        .is_err()
        {
            continue;
        }
        let expanded_wood = materials.add(Color::srgb(0.48, 0.31, 0.17));
        let shelf_material = materials.add(Color::srgb(0.67, 0.43, 0.22));
        let storage_material = materials.add(Color::srgb(0.24, 0.37, 0.24));
        commands.entity(request.target).with_children(|children| {
            children.spawn((
                Name::new("Expanded workshop machine bay"),
                StudioExpansion,
                Mesh3d(meshes.add(Cuboid::new(4.0, 0.12, 4.0))),
                MeshMaterial3d(expanded_wood),
                Transform::from_xyz(7.8, 0.0, 0.0),
            ));
            children.spawn((
                Name::new("Studio display shelf"),
                StudioExpansion,
                Mesh3d(meshes.add(Cuboid::new(2.4, 1.4, 0.5))),
                MeshMaterial3d(shelf_material),
                Transform::from_xyz(-3.8, 0.8, 1.8),
            ));
            children.spawn((
                Name::new("Expanded studio storage chest"),
                StudioExpansion,
                Mesh3d(meshes.add(Cuboid::new(1.5, 1.1, 1.0))),
                MeshMaterial3d(storage_material),
                Transform::from_xyz(4.8, 0.65, 1.55),
            ));
        });
    }
}

fn spawn_studio(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tile = PlanetTile::new(
        TileCoordinate::new(PlanetFace::PositiveY, 11, 14).expect("studio tile is within the face"),
    );
    let surface = sample_tile_surface(&tile, DEFAULT_PLANET_RADIUS)
        .expect("the configured planet radius is valid");
    let transform = surface_transform(
        SurfaceLocation::new(surface.normal, 0.05),
        Vec3::ZERO,
        DEFAULT_PLANET_RADIUS,
        &surface,
        Vec3::X,
    )
    .expect("the studio's tangent orientation is valid");

    let timber = materials.add(Color::srgb(0.38, 0.23, 0.13));
    let roof_color = Color::srgb(0.66, 0.31, 0.19);
    let roof = materials.add(roof_color);
    let faded_roof = materials.add(crate::camera_follow::obstruction_fade_material(roof_color));
    let stone = materials.add(Color::srgb(0.48, 0.47, 0.40));
    let marker = materials.add(Color::srgba(0.92, 0.76, 0.22, 0.7));
    let storage = materials.add(Color::srgb(0.30, 0.43, 0.29));

    let studio = commands
        .spawn((
            Name::new("Open-air starter studio"),
            StudioUpgrade::default(),
            Interactable::new("Expand studio (50 coins, 10 wood)"),
            transform,
            Visibility::default(),
        ))
        .id();
    commands.entity(studio).with_children(|children| {
        children.spawn((
            Name::new("Studio stone pad"),
            Mesh3d(meshes.add(Cuboid::new(12.0, 0.16, 5.0))),
            MeshMaterial3d(stone),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        for x in [-5.6, 5.6] {
            for z in [-2.1, 2.1] {
                children.spawn((
                    Name::new("Shed support post"),
                    Mesh3d(meshes.add(Cuboid::new(0.22, 3.0, 0.22))),
                    MeshMaterial3d(timber.clone()),
                    Transform::from_xyz(x, 1.5, z),
                ));
            }
        }
        children.spawn((
            Name::new("Shed roof"),
            Mesh3d(meshes.add(Cuboid::new(12.4, 0.28, 5.4))),
            MeshMaterial3d(roof.clone()),
            crate::camera_follow::CameraObstructionFade {
                opaque_material: roof,
                faded_material: faded_roof,
                radius: 6.8,
            },
            Transform::from_xyz(0.0, 3.0, 0.0),
        ));
        children.spawn((
            Name::new("Studio storage marker"),
            Mesh3d(meshes.add(Cuboid::new(1.1, 0.9, 0.8))),
            MeshMaterial3d(storage),
            Transform::from_xyz(4.8, 0.55, 1.55),
        ));
        children.spawn((
            Name::new("Studio display and placement area"),
            Mesh3d(meshes.add(Cylinder::new(1.25, 0.035))),
            MeshMaterial3d(marker),
            Transform::from_xyz(0.0, 0.105, -1.05),
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_upgrade_pays_once_and_increases_usable_storage_and_machine_space() {
        let mut upgrade = StudioUpgrade::default();
        let mut storage = StudioStorage::default();
        let mut display = StudioDisplay::default();
        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceType::Wood, STUDIO_UPGRADE_COST.quantity);
        let mut wallet = Wallet {
            coins: STUDIO_UPGRADE_COST.coins,
        };

        assert_eq!(storage.capacity(), STARTER_STORAGE_CAPACITY);
        assert_eq!(upgrade.machine_spaces(), STARTER_MACHINE_SPACES);
        upgrade_studio(
            &mut upgrade,
            &mut storage,
            &mut display,
            &mut inventory,
            &mut wallet,
        )
        .unwrap();
        assert!(upgrade.upgraded);
        assert_eq!(wallet.coins, 0);
        assert_eq!(inventory.resource_count(ResourceType::Wood), 0);
        assert_eq!(storage.capacity(), EXPANDED_STORAGE_CAPACITY);
        assert_eq!(upgrade.machine_spaces(), EXPANDED_MACHINE_SPACES);
        assert_eq!(display.capacity(), 4);
        assert_eq!(
            upgrade_studio(
                &mut upgrade,
                &mut storage,
                &mut display,
                &mut inventory,
                &mut wallet
            ),
            Err(MachineUpgradeError::AlreadyUpgraded)
        );

        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceType::Sand, 1);
        assert!(storage.deposit_resource(&mut inventory, ResourceType::Sand));
        assert_eq!(inventory.resource_count(ResourceType::Sand), 0);
        assert!(storage.contains(StoredItem::Resource(ResourceType::Sand)));
        assert!(storage.withdraw_resource(&mut inventory, ResourceType::Sand));
        assert_eq!(inventory.resource_count(ResourceType::Sand), 1);
        let ceramic = CeramicObjectId(77);
        assert!(inventory.add_ceramic(ceramic));
        assert!(display.display(&mut inventory, ceramic));
        assert!(!inventory.contains_ceramic(ceramic));
        assert_eq!(display.ceramics(), &[ceramic]);
        assert!(display.collect(&mut inventory, ceramic));
        assert!(inventory.contains_ceramic(ceramic));
    }

    #[test]
    fn failed_studio_upgrade_is_atomic_and_storage_capacity_is_enforced() {
        let mut upgrade = StudioUpgrade::default();
        let mut storage = StudioStorage::default();
        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceType::Wood, STUDIO_UPGRADE_COST.quantity);
        let mut wallet = Wallet {
            coins: STUDIO_UPGRADE_COST.coins - 1,
        };
        let mut display = StudioDisplay::default();
        assert_eq!(
            upgrade_studio(
                &mut upgrade,
                &mut storage,
                &mut display,
                &mut inventory,
                &mut wallet
            ),
            Err(MachineUpgradeError::InsufficientCoins)
        );
        assert!(!upgrade.upgraded);
        assert_eq!(storage.capacity(), STARTER_STORAGE_CAPACITY);
        assert_eq!(inventory.resource_count(ResourceType::Wood), 10);

        let resources = [
            ResourceType::CommonClay,
            ResourceType::RedClay,
            ResourceType::PaleClay,
            ResourceType::Wood,
            ResourceType::Plant,
            ResourceType::IronMineral,
            ResourceType::Shell,
            ResourceType::Sand,
        ];
        for resource in resources {
            inventory.add_resource(resource, 1);
            assert!(storage.deposit_resource(&mut inventory, resource));
        }
        for id in [101, 102] {
            let ceramic = CeramicObjectId(id);
            assert!(inventory.add_ceramic(ceramic));
            assert!(storage.deposit_ceramic(&mut inventory, ceramic));
        }
        assert_eq!(storage.used_slots(), STARTER_STORAGE_CAPACITY);
        let extra = CeramicObjectId(103);
        assert!(inventory.add_ceramic(extra));
        assert!(!storage.deposit_ceramic(&mut inventory, extra));
        assert!(inventory.contains_ceramic(extra));
    }

    #[test]
    fn headless_startup_builds_surface_aligned_studio_and_expansion_adds_layout() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.add_message::<InteractionRequested>();
        app.add_plugins(StudioPlugin);
        app.update();

        let studio = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &Name, &Transform)>();
            query
                .iter(world)
                .find(|(_, name, _)| name.as_str() == "Open-air starter studio")
                .map(|(entity, _, _)| entity)
                .expect("studio root is spawned")
        };
        let studio_transform = *app.world().get::<Transform>(studio).unwrap();
        assert!(studio_transform.translation.is_finite());
        assert!(
            (studio_transform.rotation * Vec3::Y).dot(studio_transform.translation.normalize())
                > 0.999
        );
        app.world_mut().resource_mut::<Wallet>().coins = STUDIO_UPGRADE_COST.coins;
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_resource(ResourceType::Wood, STUDIO_UPGRADE_COST.quantity);
        app.world_mut()
            .write_message(InteractionRequested { target: studio });
        app.update();
        app.update();

        let world = app.world_mut();
        let mut names = world.query::<&Name>();
        let names: Vec<_> = names.iter(world).map(Name::as_str).collect();
        assert!(names.contains(&"Studio display shelf"));
        assert!(names.contains(&"Expanded workshop machine bay"));
        assert!(names.contains(&"Expanded studio storage chest"));
        assert_eq!(
            world.resource::<StudioStorage>().capacity(),
            EXPANDED_STORAGE_CAPACITY
        );
        assert!(world.get::<StudioUpgrade>(studio).unwrap().upgraded);
        assert_eq!(world.resource::<StudioDisplay>().capacity(), 4);
        assert_eq!(
            world.get::<StudioUpgrade>(studio).unwrap().machine_spaces(),
            4
        );
    }
}
