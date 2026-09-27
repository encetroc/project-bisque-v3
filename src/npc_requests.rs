//! Authored NPC request definitions and atomic ceramic delivery.

use bevy::prelude::*;

use crate::{
    ceramics::{CeramicForm, CeramicItem, ProcessingState},
    economy::Wallet,
    inventory::{CeramicObjectId, Inventory},
    npc_placement_slots::NpcPlacementSlot,
    npc_property_upgrade::{BakeryUpgrade, apply_final_bakery_upgrade},
    npcs::{NpcCharacter, NpcFriendship, NpcProperty},
    resource_nodes::RedClayDiscovery,
    workbench::{CraftedCeramics, Workbench},
};

/// Stable identity for an authored request chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcRequestId {
    BakerTwoCups,
    BakerFinalOrder,
}

/// One finished ceramic form and quantity required by a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestRequirement {
    pub form: CeramicForm,
    pub quantity: usize,
}

/// Authored consequences applied once when a request is completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestReward {
    pub coins: u64,
    pub friendship: u8,
}

/// Data-driven definition for one request chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcRequestDefinition {
    pub id: NpcRequestId,
    pub character: NpcCharacter,
    pub requirement: RequestRequirement,
    pub additional_requirements: [RequestRequirement; 2],
    pub reward: RequestReward,
}

pub const BAKER_FINAL_ORDER: NpcRequestDefinition = NpcRequestDefinition {
    id: NpcRequestId::BakerFinalOrder,
    character: NpcCharacter::Baker,
    requirement: RequestRequirement {
        form: CeramicForm::Cup,
        quantity: 4,
    },
    additional_requirements: [
        RequestRequirement {
            form: CeramicForm::Bowl,
            quantity: 2,
        },
        RequestRequirement {
            form: CeramicForm::Vase,
            quantity: 1,
        },
    ],
    reward: RequestReward {
        coins: 0,
        friendship: 0,
    },
};

pub const BAKER_TWO_CUPS: NpcRequestDefinition = NpcRequestDefinition {
    id: NpcRequestId::BakerTwoCups,
    character: NpcCharacter::Baker,
    requirement: RequestRequirement {
        form: CeramicForm::Cup,
        quantity: 2,
    },
    additional_requirements: [
        RequestRequirement {
            form: CeramicForm::Cup,
            quantity: 0,
        },
        RequestRequirement {
            form: CeramicForm::Cup,
            quantity: 0,
        },
    ],
    reward: RequestReward {
        coins: 25,
        friendship: 10,
    },
};

/// Lifecycle for an NPC's current request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NpcRequestState {
    Available,
    Active,
    Complete,
}

/// Request chain state attached to its NPC entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcRequest {
    pub definition: NpcRequestDefinition,
    pub state: NpcRequestState,
}

/// Lifecycle for the Baker's final order, separate from the introductory request.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BakerFinalOrder {
    pub state: NpcRequestState,
}

impl Default for BakerFinalOrder {
    fn default() -> Self {
        Self {
            state: NpcRequestState::Available,
        }
    }
}

impl NpcRequest {
    pub const fn baker() -> Self {
        Self {
            definition: BAKER_TWO_CUPS,
            state: NpcRequestState::Available,
        }
    }
}

/// Begin an available request. Active and completed requests cannot be reopened.
pub fn activate_request(request: &mut NpcRequest) -> bool {
    if request.state != NpcRequestState::Available {
        return false;
    }
    request.state = NpcRequestState::Active;
    true
}

pub fn activate_baker_final_order(order: &mut BakerFinalOrder) -> bool {
    if order.state != NpcRequestState::Available {
        return false;
    }
    order.state = NpcRequestState::Active;
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryError {
    NotActive,
    InsufficientMatchingItems,
    RewardOverflow,
    ProgressionLocked,
    BakeryAlreadyUpgraded,
}

/// Deliver all required fired ceramics atomically, then apply the authored reward.
/// Failed or repeated deliveries leave request, inventory, friendship, and wallet unchanged.
pub fn deliver_request(
    request: &mut NpcRequest,
    inventory: &mut Inventory,
    ceramics: &[CeramicItem],
    friendship: &mut NpcFriendship,
    wallet: &mut Wallet,
) -> Result<Vec<CeramicObjectId>, DeliveryError> {
    if request.state != NpcRequestState::Active {
        return Err(DeliveryError::NotActive);
    }

    let mut matching = Vec::new();
    for requirement in std::iter::once(request.definition.requirement)
        .chain(request.definition.additional_requirements)
        .filter(|requirement| requirement.quantity > 0)
    {
        let selected: Vec<_> = ceramics
            .iter()
            .filter(|item| {
                item.form() == requirement.form
                    && item.state() == ProcessingState::Fired
                    && inventory.contains_ceramic(item.id)
                    && !matching.contains(&item.id)
            })
            .take(requirement.quantity)
            .map(|item| item.id)
            .collect();
        if selected.len() != requirement.quantity {
            return Err(DeliveryError::InsufficientMatchingItems);
        }
        matching.extend(selected);
    }

    let new_balance = wallet
        .coins
        .checked_add(request.definition.reward.coins)
        .ok_or(DeliveryError::RewardOverflow)?;
    let mut updated_inventory = inventory.clone();
    for id in &matching {
        if !updated_inventory.remove_ceramic(*id) {
            return Err(DeliveryError::InsufficientMatchingItems);
        }
    }

    *inventory = updated_inventory;
    wallet.coins = new_balance;
    friendship.increase(request.definition.reward.friendship);
    request.state = NpcRequestState::Complete;
    Ok(matching)
}

/// Deliver the final Baker order only after vase progression is unlocked.
/// The order and inventory commit together; the bakery upgrade is applied by the caller.
pub fn deliver_baker_final_order(
    order: &mut BakerFinalOrder,
    inventory: &mut Inventory,
    ceramics: &[CeramicItem],
    red_clay_discovered: bool,
    workbench_upgraded: bool,
    bakery_upgraded: bool,
) -> Result<Vec<CeramicItem>, DeliveryError> {
    if order.state != NpcRequestState::Active {
        return Err(DeliveryError::NotActive);
    }
    if !red_clay_discovered || !workbench_upgraded {
        return Err(DeliveryError::ProgressionLocked);
    }
    if bakery_upgraded {
        return Err(DeliveryError::BakeryAlreadyUpgraded);
    }

    let mut selected = Vec::new();
    for requirement in std::iter::once(BAKER_FINAL_ORDER.requirement)
        .chain(BAKER_FINAL_ORDER.additional_requirements)
    {
        let matching: Vec<_> = ceramics
            .iter()
            .filter(|item| {
                item.form() == requirement.form
                    && item.state() == ProcessingState::Fired
                    && inventory.contains_ceramic(item.id)
                    && !selected
                        .iter()
                        .any(|owned: &CeramicItem| owned.id == item.id)
            })
            .take(requirement.quantity)
            .copied()
            .collect();
        if matching.len() != requirement.quantity {
            return Err(DeliveryError::InsufficientMatchingItems);
        }
        selected.extend(matching);
    }

    let mut updated_inventory = inventory.clone();
    for item in &selected {
        if !updated_inventory.remove_ceramic(item.id) {
            return Err(DeliveryError::InsufficientMatchingItems);
        }
    }
    *inventory = updated_inventory;
    order.state = NpcRequestState::Complete;
    Ok(selected)
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivateNpcRequest {
    pub npc: Entity,
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliverNpcRequest {
    pub npc: Entity,
}

pub struct NpcRequestPlugin;

impl Plugin for NpcRequestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .init_resource::<Wallet>()
            .init_resource::<RedClayDiscovery>()
            .add_message::<ActivateNpcRequest>()
            .add_message::<DeliverNpcRequest>()
            .add_systems(Update, (handle_activations, handle_deliveries).chain());
    }
}

fn handle_activations(
    mut messages: MessageReader<ActivateNpcRequest>,
    mut requests: Query<(
        &NpcCharacter,
        Option<&mut NpcRequest>,
        Option<&mut BakerFinalOrder>,
    )>,
    discovery: Res<RedClayDiscovery>,
    workbenches: Query<&Workbench>,
) {
    for message in messages.read() {
        let Ok((character, mut request, mut final_order)) = requests.get_mut(message.npc) else {
            continue;
        };
        if *character != NpcCharacter::Baker {
            continue;
        }
        if let Some(request) = request.as_deref_mut()
            && request.state == NpcRequestState::Available
        {
            activate_request(request);
            continue;
        }
        let unlocked = discovery.discovered && workbenches.iter().any(|bench| bench.upgraded);
        if unlocked
            && request
                .as_deref()
                .is_some_and(|request| request.state == NpcRequestState::Complete)
            && let Some(final_order) = final_order.as_deref_mut()
        {
            activate_baker_final_order(final_order);
        }
    }
}

fn handle_deliveries(
    mut commands: Commands,
    mut messages: MessageReader<DeliverNpcRequest>,
    mut requests: Query<(
        &NpcCharacter,
        Option<&mut NpcRequest>,
        Option<&mut BakerFinalOrder>,
        &mut NpcFriendship,
    )>,
    mut inventory: ResMut<Inventory>,
    ceramics: Res<CraftedCeramics>,
    mut wallet: ResMut<Wallet>,
    discovery: Res<RedClayDiscovery>,
    workbenches: Query<&Workbench>,
    mut properties: Query<(Entity, &NpcProperty, &mut BakeryUpgrade)>,
    mut slots: Query<(Entity, &mut NpcPlacementSlot)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for message in messages.read() {
        let Ok((character, mut request, mut final_order, mut friendship)) =
            requests.get_mut(message.npc)
        else {
            continue;
        };
        if *character != NpcCharacter::Baker {
            continue;
        }

        if let Some(final_order) = final_order.as_deref_mut()
            && final_order.state == NpcRequestState::Active
            && request
                .as_deref()
                .is_some_and(|request| request.state == NpcRequestState::Complete)
        {
            let Some((property_entity, _, mut upgrade)) =
                properties.iter_mut().find(|(_, property, upgrade)| {
                    **property == NpcProperty::Bakery && !upgrade.complete
                })
            else {
                continue;
            };
            let workbench_upgraded = workbenches.iter().any(|bench| bench.upgraded);
            if let Ok(items) = deliver_baker_final_order(
                final_order,
                &mut inventory,
                &ceramics.items,
                discovery.discovered,
                workbench_upgraded,
                upgrade.complete,
            ) {
                upgrade.complete = true;
                apply_final_bakery_upgrade(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    property_entity,
                    &items,
                );
            }
            continue;
        }

        let Some(request) = request.as_deref_mut() else {
            continue;
        };
        if *character == request.definition.character
            && let Ok(delivered) = deliver_request(
                request,
                &mut inventory,
                &ceramics.items,
                &mut friendship,
                &mut wallet,
            )
        {
            // Slot index, rather than ECS iteration order, makes the display deterministic.
            let mut ordered_slots: Vec<_> = slots.iter_mut().collect();
            ordered_slots.sort_by_key(|(_, slot)| slot.index);
            for id in delivered {
                let Some(item) = ceramics.items.iter().find(|item| item.id == id).copied() else {
                    continue;
                };
                let Some((entity, slot)) = ordered_slots.iter_mut().find(|(_, slot)| {
                    slot.occupied_by.is_none() && slot.allowed_forms.contains(&item.form())
                }) else {
                    continue;
                };
                if slot.occupy(item).is_ok() {
                    let visual = crate::ceramic_visuals::spawn_ceramic_visual(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        item,
                        Transform::IDENTITY,
                    );
                    commands.entity(visual).insert(ChildOf(*entity));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ceramics::{CeramicItemTemplate, ClayMaterial, Glaze},
        inventory::INVENTORY_SLOT_COUNT,
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

    fn fixtures() -> (
        NpcRequest,
        Inventory,
        Vec<CeramicItem>,
        NpcFriendship,
        Wallet,
    ) {
        let items = vec![
            ceramic(1, CeramicForm::Cup, ProcessingState::Fired),
            ceramic(2, CeramicForm::Cup, ProcessingState::Fired),
            ceramic(3, CeramicForm::Bowl, ProcessingState::Fired),
            ceramic(4, CeramicForm::Cup, ProcessingState::Dry),
        ];
        let mut inventory = Inventory::default();
        for item in &items {
            assert!(inventory.add_ceramic(item.id));
        }
        (
            NpcRequest::baker(),
            inventory,
            items,
            NpcFriendship(2),
            Wallet { coins: 5 },
        )
    }

    #[test]
    fn request_moves_available_active_complete_and_rewards_once() {
        let (mut request, mut inventory, items, mut friendship, mut wallet) = fixtures();
        assert_eq!(request.state, NpcRequestState::Available);
        assert!(activate_request(&mut request));
        assert_eq!(request.state, NpcRequestState::Active);
        let delivered = deliver_request(
            &mut request,
            &mut inventory,
            &items,
            &mut friendship,
            &mut wallet,
        )
        .unwrap();
        assert_eq!(delivered, [CeramicObjectId(1), CeramicObjectId(2)]);
        assert_eq!(request.state, NpcRequestState::Complete);
        assert_eq!(friendship, NpcFriendship(12));
        assert_eq!(wallet.coins, 30);
        assert!(!inventory.contains_ceramic(CeramicObjectId(1)));
        assert!(!inventory.contains_ceramic(CeramicObjectId(2)));
        assert!(inventory.contains_ceramic(CeramicObjectId(3)));
        assert!(inventory.contains_ceramic(CeramicObjectId(4)));
        assert!(!activate_request(&mut request));
        assert_eq!(
            deliver_request(
                &mut request,
                &mut inventory,
                &items,
                &mut friendship,
                &mut wallet,
            ),
            Err(DeliveryError::NotActive)
        );
        assert_eq!(friendship, NpcFriendship(12));
        assert_eq!(wallet.coins, 30);
    }

    #[test]
    fn partial_matching_stock_leaves_request_and_all_inventory_unchanged() {
        let (mut request, mut inventory, items, mut friendship, mut wallet) = fixtures();
        activate_request(&mut request);
        inventory.remove_ceramic(CeramicObjectId(2));
        let before_inventory = inventory.clone();
        let before_wallet = wallet;
        assert_eq!(
            deliver_request(
                &mut request,
                &mut inventory,
                &items,
                &mut friendship,
                &mut wallet,
            ),
            Err(DeliveryError::InsufficientMatchingItems)
        );
        assert_eq!(request.state, NpcRequestState::Active);
        assert_eq!(inventory, before_inventory);
        assert_eq!(friendship, NpcFriendship(2));
        assert_eq!(wallet, before_wallet);
    }

    #[test]
    fn only_owned_fired_matching_items_count_and_reward_overflow_is_atomic() {
        let (mut request, mut inventory, items, mut friendship, mut wallet) = fixtures();
        activate_request(&mut request);
        inventory.remove_ceramic(CeramicObjectId(2));
        inventory.remove_ceramic(CeramicObjectId(1));
        inventory.add_ceramic(CeramicObjectId(3));
        inventory.add_ceramic(CeramicObjectId(4));
        assert_eq!(
            deliver_request(
                &mut request,
                &mut inventory,
                &items,
                &mut friendship,
                &mut wallet,
            ),
            Err(DeliveryError::InsufficientMatchingItems)
        );

        inventory.add_ceramic(CeramicObjectId(1));
        inventory.add_ceramic(CeramicObjectId(2));
        wallet.coins = u64::MAX;
        let before_inventory = inventory.clone();
        assert_eq!(
            deliver_request(
                &mut request,
                &mut inventory,
                &items,
                &mut friendship,
                &mut wallet,
            ),
            Err(DeliveryError::RewardOverflow)
        );
        assert_eq!(request.state, NpcRequestState::Active);
        assert_eq!(inventory, before_inventory);
        assert_eq!(friendship, NpcFriendship(2));
        assert_eq!(wallet.coins, u64::MAX);
    }

    #[test]
    fn insufficient_inventory_slots_do_not_block_request_delivery_or_duplicate_reward() {
        let (mut request, mut inventory, items, mut friendship, mut wallet) = fixtures();
        activate_request(&mut request);
        for id in 10..INVENTORY_SLOT_COUNT as u64 + 8 {
            inventory.add_ceramic(CeramicObjectId(id));
        }
        assert!(
            deliver_request(
                &mut request,
                &mut inventory,
                &items,
                &mut friendship,
                &mut wallet
            )
            .is_ok()
        );
        assert_eq!(friendship, NpcFriendship(12));
    }

    #[test]
    fn delivery_populates_deterministic_bakery_slots_once_with_persistent_cup_visuals() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(NpcRequestPlugin);

        let baker = app
            .world_mut()
            .spawn((
                NpcCharacter::Baker,
                NpcRequest::baker(),
                NpcFriendship::default(),
            ))
            .id();
        let first_slot = app
            .world_mut()
            .spawn((
                NpcPlacementSlot::bakery(0),
                Transform::from_xyz(-0.4, 0.1, 0.6),
            ))
            .id();
        let second_slot = app
            .world_mut()
            .spawn((
                NpcPlacementSlot::bakery(1),
                Transform::from_xyz(0.4, 0.1, 0.6),
            ))
            .id();
        let items = vec![
            ceramic(11, CeramicForm::Cup, ProcessingState::Fired),
            ceramic(12, CeramicForm::Cup, ProcessingState::Fired),
        ];
        {
            let mut inventory = app.world_mut().resource_mut::<Inventory>();
            inventory.add_ceramic(items[0].id);
            inventory.add_ceramic(items[1].id);
        }
        app.world_mut().resource_mut::<CraftedCeramics>().items = items.clone();
        app.world_mut()
            .write_message(ActivateNpcRequest { npc: baker });
        app.update();
        app.world_mut()
            .write_message(DeliverNpcRequest { npc: baker });
        app.update();

        let request = app.world().get::<NpcRequest>(baker).unwrap();
        assert_eq!(request.state, NpcRequestState::Complete);
        assert_eq!(
            app.world()
                .get::<NpcPlacementSlot>(first_slot)
                .unwrap()
                .occupied_by,
            Some(items[0])
        );
        assert_eq!(
            app.world()
                .get::<NpcPlacementSlot>(second_slot)
                .unwrap()
                .occupied_by,
            Some(items[1])
        );
        let mut visuals = app
            .world_mut()
            .query::<(&crate::ceramic_visuals::CeramicVisual, &ChildOf)>();
        let placed: Vec<_> = visuals
            .iter(app.world())
            .filter(|(visual, _)| items.contains(&visual.0))
            .map(|(visual, parent)| (visual.0, parent.parent()))
            .collect();
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].1, first_slot);
        assert_eq!(placed[1].1, second_slot);

        app.world_mut()
            .write_message(DeliverNpcRequest { npc: baker });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&crate::ceramic_visuals::CeramicVisual>()
                .iter(app.world())
                .filter(|visual| items.contains(&visual.0))
                .count(),
            2
        );
        assert!(app.world().entities().contains(placed[0].1));
    }

    fn final_order_stock() -> Vec<CeramicItem> {
        (0..4)
            .map(|id| ceramic(100 + id, CeramicForm::Cup, ProcessingState::Fired))
            .chain((0..2).map(|id| ceramic(104 + id, CeramicForm::Bowl, ProcessingState::Fired)))
            .chain(std::iter::once(ceramic(
                106,
                CeramicForm::Vase,
                ProcessingState::Fired,
            )))
            .collect()
    }

    #[test]
    fn final_order_requires_exact_fired_stock_and_vase_progression_atomically() {
        let items = final_order_stock();
        assert_eq!(
            BAKER_FINAL_ORDER.requirement,
            RequestRequirement {
                form: CeramicForm::Cup,
                quantity: 4,
            }
        );
        assert_eq!(
            BAKER_FINAL_ORDER.additional_requirements,
            [
                RequestRequirement {
                    form: CeramicForm::Bowl,
                    quantity: 2
                },
                RequestRequirement {
                    form: CeramicForm::Vase,
                    quantity: 1
                },
            ]
        );

        let mut order = BakerFinalOrder::default();
        assert!(activate_baker_final_order(&mut order));
        let mut inventory = Inventory::default();
        for item in &items[..6] {
            inventory.add_ceramic(item.id);
        }
        let before = inventory.clone();
        assert_eq!(
            deliver_baker_final_order(&mut order, &mut inventory, &items, true, true, false),
            Err(DeliveryError::InsufficientMatchingItems)
        );
        assert_eq!(inventory, before);
        assert_eq!(order.state, NpcRequestState::Active);

        for item in &items[6..] {
            inventory.add_ceramic(item.id);
        }
        assert_eq!(
            deliver_baker_final_order(&mut order, &mut inventory, &items, false, true, false),
            Err(DeliveryError::ProgressionLocked)
        );
        assert_eq!(inventory, before_with_vase(&before, items[6].id));
        assert_eq!(order.state, NpcRequestState::Active);

        let delivered =
            deliver_baker_final_order(&mut order, &mut inventory, &items, true, true, false)
                .unwrap();
        assert_eq!(delivered, items);
        assert_eq!(order.state, NpcRequestState::Complete);
        assert!(
            items
                .iter()
                .all(|item| !inventory.contains_ceramic(item.id))
        );
        assert_eq!(
            deliver_baker_final_order(&mut order, &mut inventory, &items, true, true, false),
            Err(DeliveryError::NotActive)
        );
    }

    fn before_with_vase(inventory: &Inventory, vase: CeramicObjectId) -> Inventory {
        let mut expected = inventory.clone();
        expected.add_ceramic(vase);
        expected
    }

    #[test]
    fn final_order_delivery_upgrades_bakery_and_populates_seven_slots_once() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(NpcRequestPlugin);

        let baker = app
            .world_mut()
            .spawn((
                NpcCharacter::Baker,
                NpcRequest {
                    definition: BAKER_TWO_CUPS,
                    state: NpcRequestState::Complete,
                },
                BakerFinalOrder::default(),
                NpcFriendship::default(),
            ))
            .id();
        let property = app
            .world_mut()
            .spawn((
                NpcProperty::Bakery,
                BakeryUpgrade::default(),
                Transform::default(),
            ))
            .id();
        let delivered_before = [
            ceramic(90, CeramicForm::Cup, ProcessingState::Fired),
            ceramic(91, CeramicForm::Cup, ProcessingState::Fired),
        ];
        for (index, item) in delivered_before.into_iter().enumerate() {
            let mut slot = NpcPlacementSlot::bakery(index);
            slot.occupy(item).unwrap();
            app.world_mut().spawn((slot, ChildOf(property)));
        }
        let workbench = app.world_mut().spawn(Workbench { upgraded: false }).id();
        let items = final_order_stock();
        for item in &items {
            app.world_mut()
                .resource_mut::<Inventory>()
                .add_ceramic(item.id);
        }
        app.world_mut().resource_mut::<CraftedCeramics>().items = items.clone();

        app.world_mut()
            .write_message(ActivateNpcRequest { npc: baker });
        app.update();
        assert_eq!(
            app.world().get::<BakerFinalOrder>(baker).unwrap().state,
            NpcRequestState::Available
        );
        app.world_mut()
            .resource_mut::<RedClayDiscovery>()
            .discovered = true;
        app.world_mut()
            .get_mut::<Workbench>(workbench)
            .unwrap()
            .upgraded = true;
        app.world_mut()
            .write_message(ActivateNpcRequest { npc: baker });
        app.update();
        assert_eq!(
            app.world().get::<BakerFinalOrder>(baker).unwrap().state,
            NpcRequestState::Active
        );
        app.world_mut()
            .resource_mut::<Inventory>()
            .remove_ceramic(items[6].id);
        let inventory_before_failed_delivery = app.world().resource::<Inventory>().clone();
        app.world_mut()
            .write_message(DeliverNpcRequest { npc: baker });
        app.update();
        assert_eq!(
            app.world().resource::<Inventory>(),
            &inventory_before_failed_delivery
        );
        assert!(!app.world().get::<BakeryUpgrade>(property).unwrap().complete);
        assert_eq!(
            app.world().get::<BakerFinalOrder>(baker).unwrap().state,
            NpcRequestState::Active
        );

        app.world_mut()
            .resource_mut::<Inventory>()
            .add_ceramic(items[6].id);
        app.world_mut()
            .write_message(DeliverNpcRequest { npc: baker });
        app.update();

        assert!(app.world().get::<BakeryUpgrade>(property).unwrap().complete);
        assert_eq!(
            app.world().get::<BakerFinalOrder>(baker).unwrap().state,
            NpcRequestState::Complete
        );
        for (offset, item) in items.iter().enumerate() {
            let slot = app
                .world_mut()
                .query::<(&NpcPlacementSlot, &ChildOf)>()
                .iter(app.world())
                .find(|(slot, parent)| {
                    parent.parent() == property
                        && slot.index == offset + 2
                        && slot.occupied_by == Some(*item)
                });
            assert!(
                slot.is_some(),
                "final order item {} has a bakery slot",
                item.id.0
            );
            assert!(
                !app.world()
                    .resource::<Inventory>()
                    .contains_ceramic(item.id)
            );
        }
        let mut expansions = app
            .world_mut()
            .query_filtered::<Entity, With<crate::npc_property_upgrade::BakeryExpansion>>();
        assert_eq!(expansions.iter(app.world()).count(), 2);
        let mut visuals = app
            .world_mut()
            .query::<&crate::ceramic_visuals::CeramicVisual>();
        assert_eq!(visuals.iter(app.world()).count(), 7);
        let mut slots = app.world_mut().query::<&NpcPlacementSlot>();
        assert_eq!(
            slots
                .iter(app.world())
                .filter(|slot| slot.occupied_by.is_some())
                .count(),
            9
        );

        app.world_mut()
            .write_message(DeliverNpcRequest { npc: baker });
        app.update();
        let mut expansions = app
            .world_mut()
            .query_filtered::<Entity, With<crate::npc_property_upgrade::BakeryExpansion>>();
        assert_eq!(expansions.iter(app.world()).count(), 2);
        let mut visuals = app
            .world_mut()
            .query::<&crate::ceramic_visuals::CeramicVisual>();
        assert_eq!(visuals.iter(app.world()).count(), 7);
    }
}
