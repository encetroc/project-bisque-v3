//! Fixed-price merchant sales and the player's coin wallet.

use bevy::prelude::*;

use crate::{
    ceramics::{CeramicItem, ProcessingState},
    interaction::Interactable,
    inventory::{CeramicObjectId, Inventory},
    planet::ResourceType,
    workbench::CraftedCeramics,
};

/// The player's spendable coin balance.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wallet {
    pub coins: u64,
}

/// Items that can be offered to a merchant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaleItem {
    Resource(ResourceType),
    Ceramic(CeramicObjectId),
}

/// One explicitly stocked basic material, including materials used by upgrades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MerchantStockEntry {
    pub resource: ResourceType,
    pub quantity: u32,
    pub unit_price: u64,
}

/// Finite, fixed merchant stock. Upgrade materials are ordinary stocked resources.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MerchantStock {
    pub entries: Vec<MerchantStockEntry>,
}

impl Default for MerchantStock {
    fn default() -> Self {
        Self {
            entries: vec![
                MerchantStockEntry {
                    resource: ResourceType::CommonClay,
                    quantity: 20,
                    unit_price: 4,
                },
                MerchantStockEntry {
                    resource: ResourceType::RedClay,
                    quantity: 20,
                    unit_price: 6,
                },
                MerchantStockEntry {
                    resource: ResourceType::PaleClay,
                    quantity: 20,
                    unit_price: 6,
                },
                MerchantStockEntry {
                    resource: ResourceType::Wood,
                    quantity: 20,
                    unit_price: 4,
                },
                MerchantStockEntry {
                    resource: ResourceType::Plant,
                    quantity: 20,
                    unit_price: 2,
                },
                MerchantStockEntry {
                    resource: ResourceType::IronMineral,
                    quantity: 20,
                    unit_price: 10,
                },
                MerchantStockEntry {
                    resource: ResourceType::Shell,
                    quantity: 20,
                    unit_price: 4,
                },
                MerchantStockEntry {
                    resource: ResourceType::Sand,
                    quantity: 20,
                    unit_price: 2,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseError {
    NotStocked,
    InvalidQuantity,
    InsufficientStock,
    InsufficientCoins,
    InsufficientInventorySpace,
    CostOverflow,
}

/// Buy an exact quantity of a stocked resource, committing stock, wallet, and
/// inventory together only after every precondition has been validated.
pub fn purchase_from_merchant(
    stock: &mut MerchantStock,
    inventory: &mut Inventory,
    wallet: &mut Wallet,
    resource: ResourceType,
    quantity: u32,
) -> Result<u64, PurchaseError> {
    if quantity == 0 {
        return Err(PurchaseError::InvalidQuantity);
    }
    let Some(entry) = stock
        .entries
        .iter()
        .find(|entry| entry.resource == resource)
    else {
        return Err(PurchaseError::NotStocked);
    };
    if entry.quantity < quantity {
        return Err(PurchaseError::InsufficientStock);
    }
    let cost = entry
        .unit_price
        .checked_mul(u64::from(quantity))
        .ok_or(PurchaseError::CostOverflow)?;
    if wallet.coins < cost {
        return Err(PurchaseError::InsufficientCoins);
    }
    let mut updated_inventory = inventory.clone();
    if updated_inventory.add_resource(resource, quantity) != 0 {
        return Err(PurchaseError::InsufficientInventorySpace);
    }

    let entry = stock
        .entries
        .iter_mut()
        .find(|entry| entry.resource == resource)
        .expect("the stocked resource was validated above");
    entry.quantity -= quantity;
    *inventory = updated_inventory;
    wallet.coins -= cost;
    Ok(cost)
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurchaseItemRequested {
    pub merchant: Entity,
    pub resource: ResourceType,
    pub quantity: u32,
}

/// Fixed unit prices. Ceramics have a fixed price by form; their material and glaze
/// do not change the price. Only fired ceramics are eligible for sale.
pub const fn resource_price(resource: ResourceType) -> u64 {
    match resource {
        ResourceType::CommonClay => 2,
        ResourceType::RedClay => 3,
        ResourceType::PaleClay => 3,
        ResourceType::Wood => 2,
        ResourceType::Plant => 1,
        ResourceType::IronMineral => 5,
        ResourceType::Shell => 2,
        ResourceType::Sand => 1,
    }
}

pub const fn ceramic_price(item: CeramicItem) -> Option<u64> {
    if !matches!(item.state(), ProcessingState::Fired) {
        return None;
    }
    Some(match item.form() {
        crate::ceramics::CeramicForm::Cup => 15,
        crate::ceramics::CeramicForm::Bowl => 20,
        crate::ceramics::CeramicForm::Vase => 30,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaleError {
    NotOwned,
    NotFired,
    NoPrice,
    BalanceOverflow,
}

/// Validates ownership and price before transferring the item and its proceeds.
/// Failed sales leave both inventory and balance unchanged.
pub fn sell_to_merchant(
    inventory: &mut Inventory,
    crafted: &[CeramicItem],
    wallet: &mut Wallet,
    item: SaleItem,
) -> Result<u64, SaleError> {
    let price = match item {
        SaleItem::Resource(resource) => {
            if inventory.resource_count(resource) == 0 {
                return Err(SaleError::NotOwned);
            }
            resource_price(resource)
        }
        SaleItem::Ceramic(id) => {
            if !inventory.contains_ceramic(id) {
                return Err(SaleError::NotOwned);
            }
            let Some(ceramic) = crafted.iter().find(|ceramic| ceramic.id == id) else {
                return Err(SaleError::NoPrice);
            };
            ceramic_price(*ceramic).ok_or(SaleError::NotFired)?
        }
    };
    let new_balance = wallet
        .coins
        .checked_add(price)
        .ok_or(SaleError::BalanceOverflow)?;

    let removed = match item {
        SaleItem::Resource(resource) => inventory.remove_resource(resource, 1) == 1,
        SaleItem::Ceramic(id) => inventory.remove_ceramic(id),
    };
    if !removed {
        return Err(SaleError::NotOwned);
    }
    wallet.coins = new_balance;
    Ok(price)
}

/// Marker for an entity that accepts sales. Merchant visuals and stock UI are
/// intentionally owned by other features.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Merchant;

/// Explicitly offer one item to a selected merchant.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItemRequested {
    pub merchant: Entity,
    pub item: SaleItem,
}

/// Emitted only after a sale transaction commits successfully.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CeramicSold {
    pub item: SaleItem,
    pub price: u64,
}

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Wallet>()
            .init_resource::<MerchantStock>()
            .add_message::<SellItemRequested>()
            .add_message::<CeramicSold>()
            .add_message::<PurchaseItemRequested>()
            .add_systems(Update, (handle_sale_requests, handle_purchase_requests));
    }
}

fn handle_purchase_requests(
    mut requests: MessageReader<PurchaseItemRequested>,
    merchants: Query<(), With<Merchant>>,
    mut stock: ResMut<MerchantStock>,
    mut inventory: ResMut<Inventory>,
    mut wallet: ResMut<Wallet>,
) {
    for request in requests.read() {
        if merchants.get(request.merchant).is_ok() {
            let _ = purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                request.resource,
                request.quantity,
            );
        }
    }
}

fn handle_sale_requests(
    mut requests: MessageReader<SellItemRequested>,
    merchants: Query<(), With<Merchant>>,
    mut inventory: ResMut<Inventory>,
    crafted: Res<CraftedCeramics>,
    mut wallet: ResMut<Wallet>,
    mut sold: MessageWriter<CeramicSold>,
) {
    for request in requests.read() {
        if merchants.get(request.merchant).is_ok()
            && let Ok(price) =
                sell_to_merchant(&mut inventory, &crafted.items, &mut wallet, request.item)
        {
            sold.write(CeramicSold {
                item: request.item,
                price,
            });
        }
    }
}

/// Helper to add the standard interaction prompt to a merchant entity.
pub fn merchant_interactable() -> Interactable {
    Interactable::new("Sell an item")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ceramics::{CeramicForm, CeramicItemTemplate, ClayMaterial, Glaze};

    fn ceramic(id: u64, state: ProcessingState, form: CeramicForm) -> CeramicItem {
        CeramicItemTemplate {
            form,
            clay: ClayMaterial::Common,
            glaze: Glaze::None,
            state,
        }
        .instantiate(CeramicObjectId(id))
    }

    fn stock_with(resource: ResourceType, quantity: u32, unit_price: u64) -> MerchantStock {
        MerchantStock {
            entries: vec![MerchantStockEntry {
                resource,
                quantity,
                unit_price,
            }],
        }
    }

    #[test]
    fn merchant_catalog_is_explicit_finite_fixed_price_stock_for_basic_materials() {
        let stock = MerchantStock::default();
        let expected = [
            ResourceType::CommonClay,
            ResourceType::RedClay,
            ResourceType::PaleClay,
            ResourceType::Wood,
            ResourceType::Plant,
            ResourceType::IronMineral,
            ResourceType::Shell,
            ResourceType::Sand,
        ];
        assert_eq!(stock.entries.len(), expected.len());
        for resource in expected {
            let entry = stock
                .entries
                .iter()
                .find(|entry| entry.resource == resource)
                .unwrap();
            assert_eq!(entry.quantity, 20);
            assert!(entry.unit_price > resource_price(resource));
        }
    }

    #[test]
    fn valid_purchase_charges_fixed_price_and_transfers_material_and_stock() {
        let mut stock = stock_with(ResourceType::IronMineral, 3, 10);
        let mut inventory = Inventory::default();
        let mut wallet = Wallet { coins: 35 };
        assert_eq!(
            purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                ResourceType::IronMineral,
                3
            ),
            Ok(30)
        );
        assert_eq!(wallet.coins, 5);
        assert_eq!(inventory.resource_count(ResourceType::IronMineral), 3);
        assert_eq!(stock.entries[0].quantity, 0);
    }

    #[test]
    fn failed_purchase_for_coins_or_inventory_space_is_atomic() {
        let mut stock = stock_with(ResourceType::Wood, 10, 4);
        let mut inventory = Inventory::default();
        let mut wallet = Wallet { coins: 7 };
        assert_eq!(
            purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                ResourceType::Wood,
                2
            ),
            Err(PurchaseError::InsufficientCoins)
        );
        assert_eq!(wallet.coins, 7);
        assert_eq!(inventory.resource_count(ResourceType::Wood), 0);
        assert_eq!(stock.entries[0].quantity, 10);

        inventory.add_resource(ResourceType::CommonClay, 99 * 20);
        wallet.coins = 100;
        assert_eq!(
            purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                ResourceType::Wood,
                1
            ),
            Err(PurchaseError::InsufficientInventorySpace)
        );
        assert_eq!(wallet.coins, 100);
        assert_eq!(inventory.resource_count(ResourceType::Wood), 0);
        assert_eq!(stock.entries[0].quantity, 10);
    }

    #[test]
    fn purchase_rejects_out_of_stock_unlisted_and_zero_quantity_requests() {
        let mut stock = stock_with(ResourceType::Sand, 2, 2);
        let mut inventory = Inventory::default();
        let mut wallet = Wallet { coins: 100 };
        assert_eq!(
            purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                ResourceType::Sand,
                3
            ),
            Err(PurchaseError::InsufficientStock)
        );
        assert_eq!(
            purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                ResourceType::IronMineral,
                1
            ),
            Err(PurchaseError::NotStocked)
        );
        assert_eq!(
            purchase_from_merchant(
                &mut stock,
                &mut inventory,
                &mut wallet,
                ResourceType::Sand,
                0
            ),
            Err(PurchaseError::InvalidQuantity)
        );
        assert_eq!(wallet.coins, 100);
        assert_eq!(inventory.resource_count(ResourceType::Sand), 0);
        assert_eq!(stock.entries[0].quantity, 2);
    }

    #[test]
    fn fixed_prices_cover_all_resource_types_and_ceramic_forms() {
        assert_eq!(resource_price(ResourceType::CommonClay), 2);
        assert_eq!(resource_price(ResourceType::RedClay), 3);
        assert_eq!(resource_price(ResourceType::PaleClay), 3);
        assert_eq!(resource_price(ResourceType::Wood), 2);
        assert_eq!(resource_price(ResourceType::Plant), 1);
        assert_eq!(resource_price(ResourceType::IronMineral), 5);
        assert_eq!(resource_price(ResourceType::Shell), 2);
        assert_eq!(resource_price(ResourceType::Sand), 1);
        assert_eq!(
            ceramic_price(ceramic(1, ProcessingState::Fired, CeramicForm::Cup)),
            Some(15)
        );
        assert_eq!(
            ceramic_price(ceramic(2, ProcessingState::Fired, CeramicForm::Bowl)),
            Some(20)
        );
        assert_eq!(
            ceramic_price(ceramic(3, ProcessingState::Fired, CeramicForm::Vase)),
            Some(30)
        );
    }

    #[test]
    fn selling_fired_ceramic_atomically_removes_item_and_credits_wallet() {
        let id = CeramicObjectId(4);
        let mut inventory = Inventory::default();
        assert!(inventory.add_ceramic(id));
        let item = ceramic(4, ProcessingState::Fired, CeramicForm::Vase);
        let mut wallet = Wallet { coins: 7 };

        assert_eq!(
            sell_to_merchant(&mut inventory, &[item], &mut wallet, SaleItem::Ceramic(id)),
            Ok(30)
        );
        assert!(!inventory.contains_ceramic(id));
        assert_eq!(wallet.coins, 37);
    }

    #[test]
    fn selling_resource_removes_one_unit_and_credits_fixed_price() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.add_resource(ResourceType::IronMineral, 3), 0);
        let mut wallet = Wallet::default();
        assert_eq!(
            sell_to_merchant(
                &mut inventory,
                &[],
                &mut wallet,
                SaleItem::Resource(ResourceType::IronMineral),
            ),
            Ok(5)
        );
        assert_eq!(inventory.resource_count(ResourceType::IronMineral), 2);
        assert_eq!(wallet.coins, 5);
    }

    #[test]
    fn unfired_missing_and_unowned_ceramics_are_rejected_without_changes() {
        let id = CeramicObjectId(5);
        let mut inventory = Inventory::default();
        assert!(inventory.add_ceramic(id));
        let greenware = ceramic(5, ProcessingState::Greenware, CeramicForm::Cup);
        let mut wallet = Wallet { coins: 2 };
        assert_eq!(
            sell_to_merchant(
                &mut inventory,
                &[greenware],
                &mut wallet,
                SaleItem::Ceramic(id)
            ),
            Err(SaleError::NotFired)
        );
        assert_eq!(
            sell_to_merchant(&mut inventory, &[], &mut wallet, SaleItem::Ceramic(id)),
            Err(SaleError::NoPrice)
        );
        assert_eq!(
            sell_to_merchant(
                &mut inventory,
                &[],
                &mut wallet,
                SaleItem::Ceramic(CeramicObjectId(99)),
            ),
            Err(SaleError::NotOwned)
        );
        assert!(inventory.contains_ceramic(id));
        assert_eq!(wallet.coins, 2);
    }

    #[test]
    fn absent_resources_and_overflow_reject_atomically() {
        let mut inventory = Inventory::default();
        let mut wallet = Wallet::default();
        assert_eq!(
            sell_to_merchant(
                &mut inventory,
                &[],
                &mut wallet,
                SaleItem::Resource(ResourceType::Sand),
            ),
            Err(SaleError::NotOwned)
        );
        inventory.add_resource(ResourceType::IronMineral, 1);
        wallet.coins = u64::MAX;
        assert_eq!(
            sell_to_merchant(
                &mut inventory,
                &[],
                &mut wallet,
                SaleItem::Resource(ResourceType::IronMineral),
            ),
            Err(SaleError::BalanceOverflow)
        );
        assert_eq!(inventory.resource_count(ResourceType::IronMineral), 1);
        assert_eq!(wallet.coins, u64::MAX);
    }

    #[test]
    fn purchase_requests_only_process_for_merchant_entities() {
        let mut app = App::new();
        app.add_plugins(EconomyPlugin)
            .init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>();
        let merchant = app.world_mut().spawn(Merchant).id();
        let non_merchant = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Wallet>().coins = 100;
        app.world_mut().write_message(PurchaseItemRequested {
            merchant: non_merchant,
            resource: ResourceType::CommonClay,
            quantity: 1,
        });
        app.world_mut().write_message(PurchaseItemRequested {
            merchant,
            resource: ResourceType::CommonClay,
            quantity: 1,
        });
        app.update();
        assert_eq!(app.world().resource::<Wallet>().coins, 96);
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::CommonClay),
            1
        );
        assert_eq!(
            app.world().resource::<MerchantStock>().entries[0].quantity,
            19
        );
    }

    #[test]
    fn sale_requests_only_process_for_merchant_entities() {
        let mut app = App::new();
        app.add_plugins(EconomyPlugin)
            .init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>();
        let merchant = app.world_mut().spawn(Merchant).id();
        let non_merchant = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_resource(ResourceType::Plant, 1);
        app.world_mut().write_message(SellItemRequested {
            merchant: non_merchant,
            item: SaleItem::Resource(ResourceType::Plant),
        });
        app.world_mut().write_message(SellItemRequested {
            merchant,
            item: SaleItem::Resource(ResourceType::Plant),
        });
        app.update();
        assert_eq!(app.world().resource::<Wallet>().coins, 1);
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .resource_count(ResourceType::Plant),
            0
        );
    }
}
