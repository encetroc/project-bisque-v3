//! Abstract daily NPC production and bounded, one-time property events.

use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::{
    game_clock::{DayTransition, GameClock},
    npc_placement_slots::NpcPropertyEventSlot,
    npcs::{NpcCharacter, NpcProperty},
};

/// Daily and lifetime production totals are abstract data, never physical stock.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcAbstractProduction {
    pub day: u64,
    pub produced_today: u32,
    pub produced_total: u64,
}

impl NpcAbstractProduction {
    pub const fn for_day(day: u64) -> Self {
        Self {
            day,
            produced_today: 0,
            produced_total: 0,
        }
    }
}

/// Authored abstract daily output; these values do not create item entities.
pub const fn daily_output(character: NpcCharacter) -> u32 {
    match character {
        NpcCharacter::Baker => 3,
        NpcCharacter::Carpenter => 1,
        NpcCharacter::Merchant => 2,
    }
}

/// Meaningful world consequence that may occupy an authored property slot once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NpcPropertyEvent {
    CarpenterAddsBench,
}

/// Events are only recorded after a compatible, empty slot accepts them.
#[derive(Resource, Debug, Default)]
pub struct AppliedNpcPropertyEvents(BTreeSet<NpcPropertyEvent>);

impl AppliedNpcPropertyEvents {
    pub fn contains(&self, event: NpcPropertyEvent) -> bool {
        self.0.contains(&event)
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcPropertyEventApplied {
    pub event: NpcPropertyEvent,
    pub day: u64,
    pub slot: Entity,
}

pub struct NpcProductionPlugin;

impl Plugin for NpcProductionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppliedNpcPropertyEvents>()
            .add_message::<NpcPropertyEventApplied>()
            .add_systems(
                Update,
                (initialize_production, update_production_and_events).chain(),
            );
    }
}

fn initialize_production(
    mut commands: Commands,
    clock: Res<GameClock>,
    npcs: Query<Entity, Added<NpcCharacter>>,
) {
    for entity in &npcs {
        commands
            .entity(entity)
            .insert(NpcAbstractProduction::for_day(clock.day()));
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct ProductionEventParams<'w, 's> {
    npcs: Query<'w, 's, (&'static NpcCharacter, &'static mut NpcAbstractProduction)>,
    slots: Query<'w, 's, (Entity, &'static mut NpcPropertyEventSlot, &'static ChildOf)>,
    properties: Query<'w, 's, &'static NpcProperty>,
    applied: ResMut<'w, AppliedNpcPropertyEvents>,
    events: MessageWriter<'w, NpcPropertyEventApplied>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

fn update_production_and_events(
    mut commands: Commands,
    mut transitions: MessageReader<DayTransition>,
    mut params: ProductionEventParams,
) {
    for transition in transitions.read() {
        for (character, mut production) in &mut params.npcs {
            if transition.day <= production.day {
                continue;
            }
            let elapsed_days = transition.day - production.day;
            production.day = transition.day;
            production.produced_today = daily_output(*character);
            production.produced_total = production
                .produced_total
                .saturating_add(u64::from(production.produced_today) * elapsed_days);
        }

        if transition.day < 2
            || params
                .applied
                .contains(NpcPropertyEvent::CarpenterAddsBench)
        {
            continue;
        }

        let mut eligible: Vec<_> = params
            .slots
            .iter_mut()
            .filter(|(_, slot, child_of)| {
                params
                    .properties
                    .get(child_of.parent())
                    .is_ok_and(|property| *property == NpcProperty::Workshop)
                    && slot.can_accept(NpcPropertyEvent::CarpenterAddsBench)
            })
            .collect();
        eligible.sort_by_key(|(_, slot, _)| slot.index);
        let Some((entity, mut slot, _)) = eligible.into_iter().next() else {
            continue;
        };
        if slot.occupy(NpcPropertyEvent::CarpenterAddsBench).is_err() {
            continue;
        }
        params
            .applied
            .0
            .insert(NpcPropertyEvent::CarpenterAddsBench);
        commands.entity(entity).with_children(|children| {
            children.spawn((
                Name::new("Carpenter's handmade bench"),
                CarpenterBench,
                Mesh3d(params.meshes.add(Cuboid::new(1.4, 0.18, 0.48))),
                MeshMaterial3d(params.materials.add(Color::srgb(0.42, 0.25, 0.13))),
                Transform::from_xyz(0.0, 0.25, 0.0),
            ));
        });
        params.events.write(NpcPropertyEventApplied {
            event: NpcPropertyEvent::CarpenterAddsBench,
            day: transition.day,
            slot: entity,
        });
    }
}

/// Marker for the single physical bench created by its explicit world event.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarpenterBench;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game_clock::GameClock, npc_placement_slots::NpcPropertyEventSlot};

    #[derive(Resource, Default)]
    struct SeenEvents(Vec<NpcPropertyEventApplied>);

    fn collect_events(
        mut messages: MessageReader<NpcPropertyEventApplied>,
        mut seen: ResMut<SeenEvents>,
    ) {
        seen.0.extend(messages.read().copied());
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<DayTransition>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(GameClock::default())
            .add_plugins(NpcProductionPlugin)
            .init_resource::<SeenEvents>()
            .add_systems(Update, collect_events.after(update_production_and_events));
        app
    }

    #[test]
    fn daily_production_updates_as_abstract_counts_only() {
        let mut app = test_app();
        let baker = app
            .world_mut()
            .spawn((NpcCharacter::Baker, NpcAbstractProduction::for_day(1)))
            .id();
        app.update();
        app.world_mut().write_message(DayTransition { day: 2 });
        app.update();

        assert_eq!(
            app.world().get::<NpcAbstractProduction>(baker),
            Some(&NpcAbstractProduction {
                day: 2,
                produced_today: 3,
                produced_total: 3,
            })
        );
        let mut benches = app.world_mut().query::<&CarpenterBench>();
        assert_eq!(benches.iter(app.world()).count(), 0);
    }

    #[test]
    fn carpenter_event_occupies_one_allowed_slot_once_and_bounds_physical_clutter() {
        let mut app = test_app();
        let workshop = app.world_mut().spawn(NpcProperty::Workshop).id();
        let slot = app
            .world_mut()
            .spawn((NpcPropertyEventSlot::workshop(0), ChildOf(workshop)))
            .id();
        app.update();

        for day in [2, 3, 4] {
            app.world_mut().write_message(DayTransition { day });
            app.update();
        }

        assert_eq!(
            app.world()
                .get::<NpcPropertyEventSlot>(slot)
                .unwrap()
                .occupied_by,
            Some(NpcPropertyEvent::CarpenterAddsBench)
        );
        assert!(
            app.world()
                .resource::<AppliedNpcPropertyEvents>()
                .contains(NpcPropertyEvent::CarpenterAddsBench)
        );
        let mut benches = app.world_mut().query::<&CarpenterBench>();
        assert_eq!(benches.iter(app.world()).count(), 1);
        assert_eq!(app.world().resource::<SeenEvents>().0.len(), 1);
        assert_eq!(app.world().resource::<SeenEvents>().0[0].slot, slot);
    }

    #[test]
    fn event_waits_for_a_valid_slot_and_never_occupies_an_ineligible_one() {
        let mut app = test_app();
        let workshop = app.world_mut().spawn(NpcProperty::Workshop).id();
        let slot = app
            .world_mut()
            .spawn((
                NpcPropertyEventSlot {
                    allowed_events: [None],
                    ..NpcPropertyEventSlot::workshop(0)
                },
                ChildOf(workshop),
            ))
            .id();
        app.update();
        app.world_mut().write_message(DayTransition { day: 2 });
        app.update();
        assert_eq!(
            app.world()
                .get::<NpcPropertyEventSlot>(slot)
                .unwrap()
                .occupied_by,
            None
        );
        assert!(
            !app.world()
                .resource::<AppliedNpcPropertyEvents>()
                .contains(NpcPropertyEvent::CarpenterAddsBench)
        );
        assert!(app.world().resource::<SeenEvents>().0.is_empty());
    }
}
