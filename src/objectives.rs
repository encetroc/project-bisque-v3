//! Authored first-playthrough objective progression and HUD.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ceramics::{CeramicForm, ProcessingState},
    economy::CeramicSold,
    inventory::{CeramicObjectId, Inventory},
    kiln::Kiln,
    npc_requests::{BakerFinalOrder, NpcRequest, NpcRequestState},
    npcs::NpcCharacter,
    resource_nodes::RedClayDiscovery,
    workbench::CraftedCeramics,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Objective {
    #[default]
    GatherClay,
    CraftCup,
    DryCup,
    FireCup,
    SellCup,
    BakerRequest,
    DiscoverHighlands,
    UpgradeKiln,
    FinalBakerOrder,
    Complete,
}

impl Objective {
    pub const fn description(self) -> &'static str {
        match self {
            Self::GatherClay => "Gather 3 common clay",
            Self::CraftCup => "Craft a cup",
            Self::DryCup => "Dry your cup",
            Self::FireCup => "Fire your cup",
            Self::SellCup => "Sell your fired cup",
            Self::BakerRequest => "Complete the Baker's request",
            Self::DiscoverHighlands => "Discover red clay in the Highlands",
            Self::UpgradeKiln => "Upgrade the kiln",
            Self::FinalBakerOrder => "Complete the Baker's final order",
            Self::Complete => "Ceramic Planet POC complete!",
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::GatherClay => Self::CraftCup,
            Self::CraftCup => Self::DryCup,
            Self::DryCup => Self::FireCup,
            Self::FireCup => Self::SellCup,
            Self::SellCup => Self::BakerRequest,
            Self::BakerRequest => Self::DiscoverHighlands,
            Self::DiscoverHighlands => Self::UpgradeKiln,
            Self::UpgradeKiln => Self::FinalBakerOrder,
            Self::FinalBakerOrder | Self::Complete => Self::Complete,
        }
    }
}

/// Saveable progress for the authored onboarding path. The selected cup ID ties
/// drying, firing, and selling to the same object rather than any ceramic activity.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ObjectiveProgress {
    pub current: Objective,
    pub tracked_cup: Option<CeramicObjectId>,
}

#[derive(Component)]
struct ObjectiveLabel;

pub struct ObjectivePlugin;

impl Plugin for ObjectivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ObjectiveProgress>()
            .add_message::<CeramicSold>()
            .add_systems(Startup, spawn_objective_label)
            .add_systems(
                Update,
                (advance_objectives, refresh_objective_label).chain(),
            );
    }
}

fn spawn_objective_label(mut commands: Commands) {
    commands.spawn((
        ObjectiveLabel,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(62),
            left: px(16),
            padding: UiRect::axes(px(12), px(8)),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.04, 0.06, 0.08, 0.88)),
    ));
}

fn advance_objectives(
    mut progress: ResMut<ObjectiveProgress>,
    inventory: Res<Inventory>,
    ceramics: Res<CraftedCeramics>,
    discovery: Res<RedClayDiscovery>,
    kilns: Query<&Kiln>,
    npcs: Query<(&NpcCharacter, Option<&NpcRequest>, Option<&BakerFinalOrder>)>,
    mut sold: MessageReader<CeramicSold>,
) {
    if progress.current == Objective::Complete {
        return;
    }
    let target = progress
        .tracked_cup
        .and_then(|id| ceramics.items.iter().find(|item| item.id == id).copied());
    let cup_sold = sold.read().any(|event| {
        matches!(event.item, crate::economy::SaleItem::Ceramic(id) if Some(id) == progress.tracked_cup)
    });
    let baker = npcs
        .iter()
        .find(|(character, _, _)| **character == NpcCharacter::Baker);
    let request_complete = baker
        .and_then(|(_, request, _)| request)
        .is_some_and(|request| request.state == NpcRequestState::Complete);
    let final_order_complete = baker
        .and_then(|(_, _, order)| order)
        .is_some_and(|order| order.state == NpcRequestState::Complete);

    let completed = match progress.current {
        Objective::GatherClay => {
            inventory.resource_count(crate::planet::ResourceType::CommonClay) >= 3
        }
        Objective::CraftCup => {
            if let Some(cup) = ceramics.items.iter().find(|item| {
                item.form() == CeramicForm::Cup
                    && item.state() == ProcessingState::Greenware
                    && inventory.contains_ceramic(item.id)
            }) {
                progress.tracked_cup = Some(cup.id);
                true
            } else {
                false
            }
        }
        Objective::DryCup => target.is_some_and(|cup| cup.state() == ProcessingState::Dry),
        Objective::FireCup => target.is_some_and(|cup| cup.state() == ProcessingState::Fired),
        Objective::SellCup => cup_sold,
        Objective::BakerRequest => request_complete,
        Objective::DiscoverHighlands => discovery.discovered,
        Objective::UpgradeKiln => kilns.iter().any(|kiln| kiln.upgraded),
        Objective::FinalBakerOrder => final_order_complete,
        Objective::Complete => false,
    };
    if completed {
        progress.current = progress.current.next();
    }
}

fn refresh_objective_label(
    progress: Res<ObjectiveProgress>,
    mut labels: Query<&mut Text, With<ObjectiveLabel>>,
) {
    if !progress.is_changed() {
        return;
    }
    let text = format!("Objective  ·  {}", progress.current.description());
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{economy::SaleItem, planet::ResourceType};

    #[test]
    fn authored_objectives_have_a_fixed_order_and_terminal_state() {
        let sequence = [
            Objective::GatherClay,
            Objective::CraftCup,
            Objective::DryCup,
            Objective::FireCup,
            Objective::SellCup,
            Objective::BakerRequest,
            Objective::DiscoverHighlands,
            Objective::UpgradeKiln,
            Objective::FinalBakerOrder,
            Objective::Complete,
        ];
        for pair in sequence.windows(2) {
            assert_eq!(pair[0].next(), pair[1]);
        }
        assert_eq!(Objective::Complete.next(), Objective::Complete);
    }

    #[test]
    fn unrelated_gathering_and_sales_cannot_advance_the_objective() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .init_resource::<RedClayDiscovery>()
            .init_resource::<ObjectiveProgress>()
            .add_message::<CeramicSold>()
            .add_systems(Update, advance_objectives);
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_resource(ResourceType::RedClay, 3);
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::GatherClay
        );

        let tracked = CeramicObjectId(8);
        *app.world_mut().resource_mut::<ObjectiveProgress>() = ObjectiveProgress {
            current: Objective::SellCup,
            tracked_cup: Some(tracked),
        };
        app.world_mut().write_message(CeramicSold {
            item: SaleItem::Ceramic(CeramicObjectId(9)),
            price: 15,
        });
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::SellCup
        );
        app.world_mut().write_message(CeramicSold {
            item: SaleItem::Ceramic(tracked),
            price: 15,
        });
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::BakerRequest
        );
    }

    #[test]
    fn headless_acceptance_path_completes_the_final_order() {
        use crate::{
            ceramics::{CeramicItemTemplate, ClayMaterial, Glaze},
            economy::{SaleItem, Wallet, sell_to_merchant},
            inventory::CeramicObjectId,
            kiln::{Kiln, upgrade_kiln},
            npc_requests::{
                BakerFinalOrder, NpcRequest, activate_baker_final_order, activate_request,
                deliver_baker_final_order, deliver_request,
            },
            npcs::{NpcCharacter, NpcFriendship},
            workbench::{CraftedCeramics, Workbench, craft_greenware},
        };

        fn fired(id: u64, form: CeramicForm, clay: ClayMaterial) -> crate::ceramics::CeramicItem {
            CeramicItemTemplate {
                form,
                clay,
                glaze: Glaze::None,
                state: ProcessingState::Fired,
            }
            .instantiate(CeramicObjectId(id))
        }

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Inventory>()
            .init_resource::<CraftedCeramics>()
            .init_resource::<RedClayDiscovery>()
            .init_resource::<ObjectiveProgress>()
            .init_resource::<Wallet>()
            .add_message::<CeramicSold>()
            .add_systems(Update, advance_objectives);

        let baker = app
            .world_mut()
            .spawn((
                NpcCharacter::Baker,
                NpcRequest::baker(),
                BakerFinalOrder::default(),
                NpcFriendship::default(),
            ))
            .id();
        let kiln = app.world_mut().spawn(Kiln::default()).id();
        let workbench = app.world_mut().spawn(Workbench::default()).id();

        // Gather and shape the tracked cup through the same transaction used by play.
        app.world_mut()
            .resource_mut::<Inventory>()
            .add_resource(ResourceType::CommonClay, 3);
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::CraftCup
        );
        let tracked = app.world_mut().resource_scope(|world, mut inventory| {
            world.resource_scope(|_, mut ceramics| {
                craft_greenware(
                    &mut inventory,
                    &mut ceramics,
                    CeramicForm::Cup,
                    ClayMaterial::Common,
                    Glaze::None,
                )
                .unwrap()
            })
        });
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().tracked_cup,
            Some(tracked)
        );
        {
            let mut ceramics = app.world_mut().resource_mut::<CraftedCeramics>();
            let cup = ceramics
                .items
                .iter_mut()
                .find(|item| item.id == tracked)
                .unwrap();
            cup.transition_to(ProcessingState::Dry).unwrap();
        }
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::FireCup
        );
        app.world_mut()
            .resource_mut::<CraftedCeramics>()
            .items
            .iter_mut()
            .find(|item| item.id == tracked)
            .unwrap()
            .transition_to(ProcessingState::Fired)
            .unwrap();
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::SellCup
        );
        let price = app.world_mut().resource_scope(|world, mut inventory| {
            let items = world.resource::<CraftedCeramics>().items.clone();
            world.resource_scope(|_, mut wallet| {
                sell_to_merchant(
                    &mut inventory,
                    &items,
                    &mut wallet,
                    SaleItem::Ceramic(tracked),
                )
                .unwrap()
            })
        });
        app.world_mut().write_message(CeramicSold {
            item: SaleItem::Ceramic(tracked),
            price,
        });
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::BakerRequest
        );

        // Complete the introductory order with two other owned, fired cups.
        let request_cups = [
            fired(10, CeramicForm::Cup, ClayMaterial::Common),
            fired(11, CeramicForm::Cup, ClayMaterial::Common),
        ];
        {
            let world = app.world_mut();
            for item in request_cups {
                world.resource_mut::<Inventory>().add_ceramic(item.id);
                world.resource_mut::<CraftedCeramics>().items.push(item);
            }
            {
                let mut request = world.get_mut::<NpcRequest>(baker).unwrap();
                assert!(activate_request(&mut request));
            }
            let items = world.resource::<CraftedCeramics>().items.clone();
            world.resource_scope(|world, mut inventory| {
                world.resource_scope(|world, mut wallet| {
                    let mut query = world.query::<(&mut NpcRequest, &mut NpcFriendship)>();
                    let (mut request, mut friendship) = query.get_mut(world, baker).unwrap();
                    assert_eq!(
                        deliver_request(
                            &mut request,
                            &mut inventory,
                            &items,
                            &mut friendship,
                            &mut wallet,
                        )
                        .unwrap()
                        .len(),
                        2
                    );
                });
            });
        }
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::DiscoverHighlands
        );

        app.world_mut()
            .resource_mut::<RedClayDiscovery>()
            .discovered = true;
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::UpgradeKiln
        );
        {
            let world = app.world_mut();
            world
                .resource_mut::<Inventory>()
                .add_resource(ResourceType::IronMineral, 3);
            world
                .resource_mut::<Inventory>()
                .add_resource(ResourceType::Wood, 5);
            world.resource_mut::<Wallet>().coins = 70;
            world.resource_scope(|world, mut inventory| {
                world.resource_scope(|world, mut wallet| {
                    assert!(
                        upgrade_kiln(
                            &mut world.get_mut::<Kiln>(kiln).unwrap(),
                            &mut inventory,
                            &mut wallet,
                        )
                        .is_ok()
                    );
                    assert!(
                        crate::workbench::upgrade_workbench(
                            &mut world.get_mut::<Workbench>(workbench).unwrap(),
                            &mut inventory,
                            &mut wallet,
                        )
                        .is_ok()
                    );
                });
            });
        }
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::FinalBakerOrder
        );

        // Highlands vase progression plus the exact fired assortment accepted by the final order.
        let final_items = [
            fired(20, CeramicForm::Cup, ClayMaterial::Red),
            fired(21, CeramicForm::Cup, ClayMaterial::Red),
            fired(22, CeramicForm::Cup, ClayMaterial::Red),
            fired(23, CeramicForm::Cup, ClayMaterial::Red),
            fired(24, CeramicForm::Bowl, ClayMaterial::Red),
            fired(25, CeramicForm::Bowl, ClayMaterial::Red),
            fired(26, CeramicForm::Vase, ClayMaterial::Red),
        ];
        {
            let world = app.world_mut();
            for item in final_items {
                world.resource_mut::<Inventory>().add_ceramic(item.id);
                world.resource_mut::<CraftedCeramics>().items.push(item);
            }
            {
                let mut order = world.get_mut::<BakerFinalOrder>(baker).unwrap();
                assert!(activate_baker_final_order(&mut order));
            }
            let items = world.resource::<CraftedCeramics>().items.clone();
            let delivered = world.resource_scope(|world, mut inventory| {
                let mut order = world.get_mut::<BakerFinalOrder>(baker).unwrap();
                deliver_baker_final_order(&mut order, &mut inventory, &items, true, true, false)
                    .unwrap()
            });
            assert_eq!(delivered.len(), 7);
        }
        app.update();
        assert_eq!(
            app.world().resource::<ObjectiveProgress>().current,
            Objective::Complete
        );
    }

    #[test]
    fn objective_state_round_trips_for_existing_save_data() {
        let progress = ObjectiveProgress {
            current: Objective::UpgradeKiln,
            tracked_cup: Some(CeramicObjectId(42)),
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert_eq!(
            serde_json::from_str::<ObjectiveProgress>(&json).unwrap(),
            progress
        );
        let older = r#"{"current":"GatherClay","tracked_cup":null}"#;
        assert_eq!(
            serde_json::from_str::<ObjectiveProgress>(older).unwrap(),
            ObjectiveProgress::default()
        );
    }
}
