//! Authored NPC request definitions and atomic ceramic delivery.

use bevy::prelude::*;

use crate::{
    ceramics::{CeramicForm, CeramicItem, ProcessingState},
    economy::Wallet,
    inventory::{CeramicObjectId, Inventory},
    npcs::{NpcCharacter, NpcFriendship},
    workbench::CraftedCeramics,
};

/// Stable identity for an authored request chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcRequestId {
    BakerTwoCups,
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
    pub reward: RequestReward,
}

pub const BAKER_TWO_CUPS: NpcRequestDefinition = NpcRequestDefinition {
    id: NpcRequestId::BakerTwoCups,
    character: NpcCharacter::Baker,
    requirement: RequestRequirement {
        form: CeramicForm::Cup,
        quantity: 2,
    },
    reward: RequestReward {
        coins: 25,
        friendship: 10,
    },
};

/// Lifecycle for an NPC's current request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryError {
    NotActive,
    InsufficientMatchingItems,
    RewardOverflow,
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

    let requirement = request.definition.requirement;
    let matching: Vec<_> = ceramics
        .iter()
        .filter(|item| {
            item.form() == requirement.form
                && item.state() == ProcessingState::Fired
                && inventory.contains_ceramic(item.id)
        })
        .take(requirement.quantity)
        .map(|item| item.id)
        .collect();
    if matching.len() != requirement.quantity {
        return Err(DeliveryError::InsufficientMatchingItems);
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
            .add_message::<ActivateNpcRequest>()
            .add_message::<DeliverNpcRequest>()
            .add_systems(Update, (handle_activations, handle_deliveries).chain());
    }
}

fn handle_activations(
    mut messages: MessageReader<ActivateNpcRequest>,
    mut requests: Query<(&NpcCharacter, &mut NpcRequest)>,
) {
    for message in messages.read() {
        if let Ok((character, mut request)) = requests.get_mut(message.npc)
            && *character == request.definition.character
        {
            activate_request(&mut request);
        }
    }
}

fn handle_deliveries(
    mut messages: MessageReader<DeliverNpcRequest>,
    mut requests: Query<(&NpcCharacter, &mut NpcRequest, &mut NpcFriendship)>,
    mut inventory: ResMut<Inventory>,
    ceramics: Res<CraftedCeramics>,
    mut wallet: ResMut<Wallet>,
) {
    for message in messages.read() {
        let Ok((character, mut request, mut friendship)) = requests.get_mut(message.npc) else {
            continue;
        };
        if *character == request.definition.character {
            let _ = deliver_request(
                &mut request,
                &mut inventory,
                &ceramics.items,
                &mut friendship,
                &mut wallet,
            );
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
}
