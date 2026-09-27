//! Compact day/time display for the current game clock.

use bevy::prelude::*;

use crate::game_clock::GameClock;

#[derive(Component)]
struct DayTimeLabel;

pub struct DayTimeHudPlugin;

impl Plugin for DayTimeHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_day_time_label)
            .add_systems(Update, refresh_day_time_label);
    }
}

fn spawn_day_time_label(mut commands: Commands) {
    commands.spawn((
        DayTimeLabel,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            padding: UiRect::axes(px(12), px(8)),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.04, 0.06, 0.08, 0.88)),
    ));
}

fn format_day_time(clock: &GameClock) -> String {
    let minute_of_day = clock.minute_of_day().floor() as u32;
    let hour = minute_of_day / 60;
    let minute = minute_of_day % 60;
    let period = if hour < 12 { "AM" } else { "PM" };
    let display_hour = match hour % 12 {
        0 => 12,
        hour => hour,
    };
    format!(
        "Day {}  ·  {display_hour}:{minute:02} {period}",
        clock.day()
    )
}

fn refresh_day_time_label(clock: Res<GameClock>, mut labels: Query<&mut Text, With<DayTimeLabel>>) {
    if !clock.is_changed() {
        return;
    }
    let text = format_day_time(&clock);
    for mut label in &mut labels {
        label.0.clone_from(&text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_label_tracks_minutes_and_day_rollover() {
        let mut clock = GameClock::default();
        assert_eq!(format_day_time(&clock), "Day 1  ·  6:00 AM");

        clock.advance_real_seconds(9.0);
        assert_eq!(format_day_time(&clock), "Day 1  ·  6:54 AM");

        clock.restore(2, 13.0 * 60.0 + 7.0);
        assert_eq!(format_day_time(&clock), "Day 2  ·  1:07 PM");
    }

    #[test]
    fn clock_label_updates_headlessly() {
        let mut app = App::new();
        app.init_resource::<GameClock>()
            .add_systems(Update, refresh_day_time_label);
        let label = app.world_mut().spawn((DayTimeLabel, Text::new(""))).id();

        app.update();
        assert_eq!(
            app.world().get::<Text>(label).unwrap().0,
            "Day 1  ·  6:00 AM"
        );

        app.world_mut()
            .resource_mut::<GameClock>()
            .advance_real_seconds(10.0);
        app.update();
        assert_eq!(
            app.world().get::<Text>(label).unwrap().0,
            "Day 1  ·  7:00 AM"
        );
    }
}
