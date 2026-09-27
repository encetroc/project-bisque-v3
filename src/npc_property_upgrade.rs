//! The Baker's one-time bakery expansion and its persistent ceramic display.

use bevy::prelude::*;

use crate::{
    ceramics::{CeramicForm, CeramicItem, ProcessingState},
    economy::Wallet,
    inventory::Inventory,
    npc_placement_slots::NpcPlacementSlot,
    npcs::NpcProperty,
    workbench::CraftedCeramics,
};

pub const BAKERY_UPGRADE_CUPS: usize = 4;
pub const BAKERY_UPGRADE_BOWLS: usize = 2;
pub const BAKERY_UPGRADE_COST: u64 = 50;

/// State attached to the bakery property. There is no second upgrade tier.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BakeryUpgrade {
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BakeryUpgradeError {
    AlreadyComplete,
    InsufficientCoins,
    InsufficientMatchingCeramics,
    RewardOverflow,
}

/// Charge the authored price and consume four fired cups plus two fired bowls atomically.
pub fn complete_bakery_upgrade(
    upgrade: &mut BakeryUpgrade,
    inventory: &mut Inventory,
    ceramics: &[CeramicItem],
    wallet: &mut Wallet,
) -> Result<Vec<CeramicItem>, BakeryUpgradeError> {
    if upgrade.complete {
        return Err(BakeryUpgradeError::AlreadyComplete);
    }
    if wallet.coins < BAKERY_UPGRADE_COST {
        return Err(BakeryUpgradeError::InsufficientCoins);
    }

    let mut selected: Vec<CeramicItem> =
        Vec::with_capacity(BAKERY_UPGRADE_CUPS + BAKERY_UPGRADE_BOWLS);
    for (form, quantity) in [
        (CeramicForm::Cup, BAKERY_UPGRADE_CUPS),
        (CeramicForm::Bowl, BAKERY_UPGRADE_BOWLS),
    ] {
        for _ in 0..quantity {
            let candidate = ceramics.iter().find(|item| {
                item.form() == form
                    && item.state() == ProcessingState::Fired
                    && inventory.contains_ceramic(item.id)
                    && !selected.iter().any(|selected| selected.id == item.id)
            });
            let Some(candidate) = candidate else {
                return Err(BakeryUpgradeError::InsufficientMatchingCeramics);
            };
            selected.push(*candidate);
        }
        if selected.iter().filter(|item| item.form() == form).count() != quantity {
            return Err(BakeryUpgradeError::InsufficientMatchingCeramics);
        }
    }

    // Apply to a copy so even malformed duplicate data cannot leave a partial transaction.
    let mut updated_inventory = inventory.clone();
    for item in &selected {
        if !updated_inventory.remove_ceramic(item.id) {
            return Err(BakeryUpgradeError::InsufficientMatchingCeramics);
        }
    }
    *inventory = updated_inventory;
    wallet.coins -= BAKERY_UPGRADE_COST;
    upgrade.complete = true;
    Ok(selected)
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeNpcProperty {
    pub property: Entity,
}

/// Marker for the distinct expansion pieces added to the upgraded bakery.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BakeryExpansion;

pub struct NpcPropertyUpgradePlugin;

impl Plugin for NpcPropertyUpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .init_resource::<Wallet>()
            .add_message::<UpgradeNpcProperty>()
            .add_systems(Update, handle_upgrade_requests);
    }
}

fn handle_upgrade_requests(
    mut commands: Commands,
    mut messages: MessageReader<UpgradeNpcProperty>,
    mut properties: Query<(&NpcProperty, &mut BakeryUpgrade)>,
    mut inventory: ResMut<Inventory>,
    ceramics: Res<CraftedCeramics>,
    mut wallet: ResMut<Wallet>,
    mut slots: Query<(&mut NpcPlacementSlot, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for message in messages.read() {
        let Ok((property, mut upgrade)) = properties.get_mut(message.property) else {
            continue;
        };
        if *property != NpcProperty::Bakery {
            continue;
        }
        let Ok(items) =
            complete_bakery_upgrade(&mut upgrade, &mut inventory, &ceramics.items, &mut wallet)
        else {
            continue;
        };

        let positions = bakery_slot_positions();
        for (slot, mut transform) in &mut slots {
            if let Some(position) = positions.get(slot.index) {
                transform.translation = *position;
            }
        }

        let wall = materials.add(Color::srgb(0.89, 0.70, 0.43));
        let roof = materials.add(Color::srgb(0.70, 0.27, 0.17));
        commands.entity(message.property).with_children(|children| {
            children.spawn((
                Name::new("Bakery expanded side room"),
                BakeryExpansion,
                Mesh3d(meshes.add(Cuboid::new(2.5, 1.8, 1.7))),
                MeshMaterial3d(wall),
                Transform::from_xyz(1.45, 0.9, -0.15),
            ));
            children.spawn((
                Name::new("Bakery expanded roof"),
                BakeryExpansion,
                Mesh3d(meshes.add(Cuboid::new(2.9, 0.18, 2.5))),
                MeshMaterial3d(roof),
                Transform::from_xyz(0.72, 1.98, -0.02),
            ));
        });
        for (offset, item) in items.into_iter().enumerate() {
            let index = offset + 2;
            let mut slot = NpcPlacementSlot::bakery(index);
            // The authored requirement only selects allowed, fired cups and bowls.
            slot.occupy(item)
                .expect("upgrade ceramics fit bakery display slots");
            let slot_entity = commands
                .spawn((
                    Name::new(format!("Bakery ceramic slot {index}")),
                    slot,
                    Transform::from_translation(positions[index]),
                    ChildOf(message.property),
                ))
                .id();
            let visual = crate::ceramic_visuals::spawn_ceramic_visual(
                &mut commands,
                &mut meshes,
                &mut materials,
                item,
                Transform::IDENTITY,
            );
            commands.entity(visual).insert(ChildOf(slot_entity));
        }
    }
}

fn bakery_slot_positions() -> [Vec3; 8] {
    [
        Vec3::new(-0.70, 0.12, 0.62),
        Vec3::new(0.00, 0.12, 0.62),
        Vec3::new(0.70, 0.12, 0.62),
        Vec3::new(1.35, 0.12, 0.62),
        Vec3::new(-0.70, 0.12, -0.05),
        Vec3::new(0.00, 0.12, -0.05),
        Vec3::new(0.70, 0.12, -0.05),
        Vec3::new(1.35, 0.12, -0.05),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ceramics::{CeramicItemTemplate, ClayMaterial, Glaze},
        inventory::CeramicObjectId,
        npc_requests::{ActivateNpcRequest, DeliverNpcRequest, NpcRequest, NpcRequestState},
        npcs::NpcCharacter,
    };

    fn ceramic(id: u64, form: CeramicForm, state: ProcessingState) -> CeramicItem {
        CeramicItemTemplate {
            form,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
            state,
        }
        .instantiate(CeramicObjectId(id))
    }

    fn stock() -> Vec<CeramicItem> {
        (0..4)
            .map(|id| ceramic(id, CeramicForm::Cup, ProcessingState::Fired))
            .chain((4..6).map(|id| ceramic(id, CeramicForm::Bowl, ProcessingState::Fired)))
            .collect()
    }

    fn owned(items: &[CeramicItem]) -> Inventory {
        let mut inventory = Inventory::default();
        for item in items {
            assert!(inventory.add_ceramic(item.id));
        }
        inventory
    }

    #[test]
    fn upgrade_is_blocked_atomically_then_succeeds_once() {
        let items = stock();
        let mut upgrade = BakeryUpgrade::default();
        let mut inventory = owned(&items);
        let before = inventory.clone();
        let mut wallet = Wallet {
            coins: BAKERY_UPGRADE_COST - 1,
        };
        assert_eq!(
            complete_bakery_upgrade(&mut upgrade, &mut inventory, &items, &mut wallet),
            Err(BakeryUpgradeError::InsufficientCoins)
        );
        assert_eq!(inventory, before);
        assert_eq!(wallet.coins, BAKERY_UPGRADE_COST - 1);
        assert!(!upgrade.complete);
        wallet.coins += 1;
        assert_eq!(
            complete_bakery_upgrade(&mut upgrade, &mut inventory, &items, &mut wallet),
            Ok(items.clone())
        );
        assert!(upgrade.complete);
        assert_eq!(wallet.coins, 0);
        assert!(
            items
                .iter()
                .all(|item| !inventory.contains_ceramic(item.id))
        );
        assert_eq!(
            complete_bakery_upgrade(&mut upgrade, &mut inventory, &items, &mut wallet),
            Err(BakeryUpgradeError::AlreadyComplete)
        );
    }

    #[test]
    fn missing_or_unfired_requirements_do_not_mutate_anything() {
        let mut items = stock();
        items[5] = ceramic(5, CeramicForm::Bowl, ProcessingState::Dry);
        let mut inventory = owned(&items);
        let before = inventory.clone();
        let mut upgrade = BakeryUpgrade::default();
        let mut wallet = Wallet { coins: 100 };
        assert_eq!(
            complete_bakery_upgrade(&mut upgrade, &mut inventory, &items, &mut wallet),
            Err(BakeryUpgradeError::InsufficientMatchingCeramics)
        );
        assert_eq!(inventory, before);
        assert_eq!(wallet.coins, 100);
        assert!(!upgrade.complete);
    }

    #[test]
    fn integration_upgrade_expands_property_and_keeps_old_and_new_deliveries_visible() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(crate::npc_requests::NpcRequestPlugin)
            .add_plugins(NpcPropertyUpgradePlugin);
        let property = app
            .world_mut()
            .spawn((NpcProperty::Bakery, BakeryUpgrade::default()))
            .id();
        let baker = app
            .world_mut()
            .spawn((
                NpcCharacter::Baker,
                NpcRequest::baker(),
                crate::npcs::NpcFriendship::default(),
            ))
            .id();
        for index in 0..2 {
            app.world_mut().spawn((
                NpcPlacementSlot::bakery(index),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }
        let old_items = [
            ceramic(20, CeramicForm::Cup, ProcessingState::Fired),
            ceramic(21, CeramicForm::Cup, ProcessingState::Fired),
        ];
        for item in old_items {
            app.world_mut()
                .resource_mut::<Inventory>()
                .add_ceramic(item.id);
        }
        app.world_mut().resource_mut::<CraftedCeramics>().items = old_items.to_vec();
        app.world_mut()
            .write_message(ActivateNpcRequest { npc: baker });
        app.update();
        app.world_mut()
            .write_message(DeliverNpcRequest { npc: baker });
        app.update();

        let upgrade_items = stock();
        for item in &upgrade_items {
            app.world_mut()
                .resource_mut::<Inventory>()
                .add_ceramic(item.id);
        }
        app.world_mut()
            .resource_mut::<CraftedCeramics>()
            .items
            .extend(upgrade_items.iter().copied());
        app.world_mut().resource_mut::<Wallet>().coins = BAKERY_UPGRADE_COST;
        app.world_mut()
            .write_message(UpgradeNpcProperty { property });
        app.update();
        assert!(app.world().get::<BakeryUpgrade>(property).unwrap().complete);
        assert_eq!(
            app.world().get::<NpcRequest>(baker).unwrap().state,
            NpcRequestState::Complete
        );
        let mut slots = app.world_mut().query::<&NpcPlacementSlot>();
        let occupied: Vec<_> = slots
            .iter(app.world())
            .filter_map(|slot| slot.occupied_by)
            .collect();
        assert_eq!(occupied.len(), 8);
        assert!(old_items.iter().all(|item| occupied.contains(item)));
        assert!(upgrade_items.iter().all(|item| occupied.contains(item)));
        let mut expansions = app
            .world_mut()
            .query_filtered::<Entity, With<BakeryExpansion>>();
        assert_eq!(expansions.iter(app.world()).count(), 2);
        let mut visuals = app
            .world_mut()
            .query::<&crate::ceramic_visuals::CeramicVisual>();
        assert_eq!(visuals.iter(app.world()).count(), 8);
    }
}
