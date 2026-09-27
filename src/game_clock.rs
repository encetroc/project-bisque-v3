//! Game-time clock and day-boundary notifications.

use bevy::prelude::*;

/// Game minutes elapsed during one real-world second.
pub const GAME_MINUTES_PER_REAL_SECOND: f64 = 6.0;
/// A complete game day contains 24 hours.
pub const MINUTES_PER_GAME_DAY: f64 = 24.0 * 60.0;
/// Sleeping wakes the player at 6:00 AM on the next day.
pub const MORNING_MINUTE: f64 = 6.0 * 60.0;

/// The current in-game day and time of day. A new game starts at day 1, 6:00 AM.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GameClock {
    day: u64,
    minute_of_day: f64,
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            day: 1,
            minute_of_day: MORNING_MINUTE,
        }
    }
}

impl GameClock {
    pub const fn day(&self) -> u64 {
        self.day
    }

    /// Minutes after midnight, including fractional minutes.
    pub const fn minute_of_day(&self) -> f64 {
        self.minute_of_day
    }

    pub(crate) fn restore(&mut self, day: u64, minute_of_day: f64) -> bool {
        if day == 0
            || !minute_of_day.is_finite()
            || !(0.0..MINUTES_PER_GAME_DAY).contains(&minute_of_day)
        {
            return false;
        }
        self.day = day;
        self.minute_of_day = minute_of_day;
        true
    }

    /// Advance by real seconds and return one notification for every day crossed.
    pub fn advance_real_seconds(&mut self, seconds: f64) -> Vec<DayTransition> {
        if !seconds.is_finite() || seconds <= 0.0 {
            return Vec::new();
        }

        self.minute_of_day += seconds * GAME_MINUTES_PER_REAL_SECOND;
        let mut transitions = Vec::new();
        while self.minute_of_day >= MINUTES_PER_GAME_DAY {
            self.minute_of_day -= MINUTES_PER_GAME_DAY;
            self.day = self.day.saturating_add(1);
            transitions.push(DayTransition { day: self.day });
        }
        transitions
    }

    /// Move directly to 6:00 AM on the next day and report that day transition.
    pub fn sleep_to_next_morning(&mut self) -> DayTransition {
        self.day = self.day.saturating_add(1);
        self.minute_of_day = MORNING_MINUTE;
        DayTransition { day: self.day }
    }
}

/// Request the game clock to advance directly to the next morning.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SleepToNextMorning;

/// Published once for each day boundary crossed by normal time or sleep.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayTransition {
    pub day: u64,
}

/// Advances game time using unscaled real time and publishes day transitions.
pub struct GameClockPlugin;

impl Plugin for GameClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameClock>()
            .add_message::<SleepToNextMorning>()
            .add_message::<DayTransition>()
            .add_systems(Update, (advance_game_clock, process_sleep_requests).chain());
    }
}

fn process_sleep_requests(
    mut requests: MessageReader<SleepToNextMorning>,
    mut clock: ResMut<GameClock>,
    mut transitions: MessageWriter<DayTransition>,
) {
    for _ in requests.read() {
        transitions.write(clock.sleep_to_next_morning());
    }
}

fn advance_game_clock(
    real_time: Res<Time<Real>>,
    mut clock: ResMut<GameClock>,
    mut transitions: MessageWriter<DayTransition>,
) {
    for transition in clock.advance_real_seconds(real_time.delta_secs_f64()) {
        transitions.write(transition);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_conversion_advances_six_game_minutes_per_real_second() {
        let mut clock = GameClock::default();
        assert!(clock.advance_real_seconds(1.0).is_empty());
        assert_eq!(clock.day(), 1);
        assert_eq!(clock.minute_of_day(), MORNING_MINUTE + 6.0);
    }

    #[test]
    fn rollover_emits_exactly_one_transition_for_each_crossed_day() {
        let mut clock = GameClock::default();
        let transitions = clock.advance_real_seconds(8.0 * 60.0);
        assert_eq!(
            transitions,
            [DayTransition { day: 2 }, DayTransition { day: 3 },]
        );
        assert_eq!(clock.day(), 3);
        assert_eq!(clock.minute_of_day(), MORNING_MINUTE);
        assert!(clock.advance_real_seconds(0.0).is_empty());
    }

    #[test]
    fn sleep_wakes_at_six_on_the_next_day_and_emits_one_transition() {
        let mut clock = GameClock::default();
        clock.advance_real_seconds(60.0);
        let transition = clock.sleep_to_next_morning();
        assert_eq!(transition, DayTransition { day: 2 });
        assert_eq!(clock.day(), 2);
        assert_eq!(clock.minute_of_day(), MORNING_MINUTE);
    }

    #[derive(Resource, Default)]
    struct SeenTransitions(Vec<DayTransition>);

    fn collect_transitions(
        mut transitions: MessageReader<DayTransition>,
        mut seen: ResMut<SeenTransitions>,
    ) {
        seen.0.extend(transitions.read().copied());
    }

    #[test]
    fn plugin_publishes_one_message_for_each_day_crossed_by_sleep() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(GameClockPlugin)
            .init_resource::<SeenTransitions>()
            .add_systems(Update, collect_transitions.after(process_sleep_requests));
        app.update();
        app.world_mut()
            .resource_mut::<Messages<SleepToNextMorning>>()
            .write(SleepToNextMorning);
        app.update();

        assert_eq!(
            app.world().resource::<SeenTransitions>().0,
            [DayTransition { day: 2 }]
        );
        assert_eq!(
            app.world().resource::<GameClock>().minute_of_day(),
            MORNING_MINUTE
        );
    }

    #[test]
    fn invalid_or_negative_elapsed_time_does_not_change_the_clock() {
        let mut clock = GameClock::default();
        assert!(clock.advance_real_seconds(f64::NAN).is_empty());
        assert!(clock.advance_real_seconds(-1.0).is_empty());
        assert_eq!(clock, GameClock::default());
    }
}
