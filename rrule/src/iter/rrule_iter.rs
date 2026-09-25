use super::counter_date::DateTimeIter;
use super::utils::add_time_to_date;
use super::{build_pos_list, utils::date_from_ordinal, IterInfo, MAX_ITER_LOOP};
use crate::core::{get_hour, get_minute, get_second};
use crate::validator::YEAR_RANGE;
use crate::{Frequency, RRule};
use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::Zoned;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub(crate) struct RRuleIter {
    /// Date the iterator is currently at.
    pub(crate) counter_date: DateTimeIter,
    pub(crate) ii: IterInfo,
    pub(crate) timeset: Vec<Time>,
    pub(crate) dt_start: Zoned,
    /// Buffer of datetimes is not yet yielded
    pub(crate) buffer: VecDeque<Zoned>,
    /// Indicate of iterator should not return more items.
    /// Once set `true` is will always return `None`.
    pub(crate) finished: bool,
    /// Number of events that should still be generated before the end.
    /// Counter always goes down after each iteration.
    pub(crate) count: Option<u32>,
    /// If the iterator should be using iterator limits.
    pub(crate) limited: bool,
    /// If the iterator has been stopped by the iterator limits.
    pub(crate) was_limited: bool,
}

impl RRuleIter {
    pub(crate) fn new(rrule: &RRule, dt_start: &Zoned, limited: bool) -> Self {
        // A rule may be validated against one DTSTART and iterated from another.
        // Outside the year range the year tables cannot be built, so there is
        // nothing to generate; build them for a stand-in date and finish at once.
        let in_range = YEAR_RANGE.contains(&i32::from(dt_start.year()));
        let ii = if in_range {
            IterInfo::new(rrule, dt_start)
        } else {
            IterInfo::new(rrule, &jiff::Timestamp::UNIX_EPOCH.to_zoned(TimeZone::UTC))
        };

        let hour = get_hour(dt_start);
        let minute = get_minute(dt_start);
        let second = get_second(dt_start);
        let timeset = ii.get_timeset(hour, minute, second);
        let count = ii.rrule().count;

        Self {
            counter_date: dt_start.into(),
            ii,
            timeset,
            dt_start: dt_start.clone(),
            buffer: VecDeque::new(),
            finished: !in_range,
            count,
            limited,
            was_limited: false,
        }
    }

    /// Attempts to add a date to the result. Returns `true` if we should
    /// terminate the iteration.
    fn try_add_datetime(
        dt: Zoned,
        rrule: &RRule,
        count: &mut Option<u32>,
        buffer: &mut VecDeque<Zoned>,
        dt_start: &Zoned,
    ) -> bool {
        if matches!(&rrule.until, Some(until) if dt > *until) {
            // We can break because `pos_list` is sorted and
            // all the next dates will only be larger than `until`.
            return true;
        }

        if dt >= *dt_start {
            buffer.push_back(dt);

            if let Some(count) = count {
                *count -= 1;

                if *count == 0 {
                    return true;
                }
            }
        }
        false
    }

    /// Generates a list of dates that will be added to the buffer.
    /// Returns true if finished, no more items should/can be returned.
    fn generate(&mut self) -> bool {
        // Do early check if done (if known)
        if self.finished {
            return true;
        }
        // Check if the count is set, and if 0
        if matches!(self.count, Some(count) if count == 0) {
            return true;
        }

        let rrule = self.ii.rrule();

        if rrule.interval == 0 {
            return true;
        }

        let mut loop_counter: u32 = 0;
        // Loop until there is at least 1 item in the buffer.
        while self.buffer.is_empty() {
            // Prevent infinite loops
            if self.limited {
                loop_counter += 1;
                if loop_counter >= MAX_ITER_LOOP {
                    self.finished = true;
                    self.was_limited = true;
                    log::warn!(
                        "Reached max loop counter (`{}`). \
                    See 'validator limits' in docs for more info.",
                        MAX_ITER_LOOP
                    );
                    return true;
                }
            }
            let rrule = self.ii.rrule();

            let dayset = self.ii.get_dayset(
                rrule.freq,
                self.counter_date.year,
                self.counter_date.month,
                self.counter_date.day,
            );

            let tz = self.dt_start.time_zone();

            if rrule.by_set_pos.is_empty() {
                // Loop over `start..end`
                for current_day in &dayset {
                    let current_day = i64::try_from(*current_day).expect(
                        "We control the dayset, and we know that it will always fit within an i64",
                    );
                    let year_ordinal = self.ii.year_ordinal();
                    // Ordinal conversion uses UTC: if we apply local-TZ here, then
                    // just below we'll end up double-applying.
                    // Past either end of jiff's date range there is no such day.
                    let Some(date) = date_from_ordinal(year_ordinal + current_day) else {
                        continue;
                    };
                    for time in &self.timeset {
                        let Some(dt) = add_time_to_date(tz, date, *time) else {
                            continue;
                        };
                        if Self::try_add_datetime(
                            dt,
                            rrule,
                            &mut self.count,
                            &mut self.buffer,
                            &self.dt_start,
                        ) {
                            return true;
                        }
                    }
                }
            } else {
                let pos_list = build_pos_list(
                    &rrule.by_set_pos,
                    &dayset,
                    &self.timeset,
                    self.ii.year_ordinal(),
                    self.dt_start.time_zone(),
                );
                for dt in pos_list {
                    if Self::try_add_datetime(
                        dt,
                        rrule,
                        &mut self.count,
                        &mut self.buffer,
                        &self.dt_start,
                    ) {
                        return true;
                    }
                }
            }

            let increment_day = dayset.is_empty();
            if self.counter_date.increment(rrule, increment_day).is_err() {
                self.finished = true;
                return true;
            }

            if matches!(
                rrule.freq,
                Frequency::Hourly | Frequency::Minutely | Frequency::Secondly
            ) {
                let hour =
                    u8::try_from(self.counter_date.hour).expect("range 0-23 is covered by u8");
                let minute =
                    u8::try_from(self.counter_date.minute).expect("range 0-59 is covered by u8");
                let second =
                    u8::try_from(self.counter_date.second).expect("range 0-59 is covered by u8");
                self.timeset = self.ii.get_timeset_unchecked(hour, minute, second);
            }

            self.ii.rebuild(&self.counter_date);

            if self.period_starts_after_until() {
                self.finished = true;
                return true;
            }
        }

        // Indicate that there might be more items on the next iteration.
        false
    }
}

impl RRuleIter {
    /// Whether nothing in the counter's current period can be at or before UNTIL.
    ///
    /// Candidates are otherwise only compared with UNTIL once generated, so a rule
    /// that has stopped matching would walk on to the end of the year range.
    fn period_starts_after_until(&self) -> bool {
        let rrule = self.ii.rrule();
        let Some(until) = &rrule.until else {
            return false;
        };
        let c = &self.counter_date;
        // The period's first day: yearly and monthly periods cover the whole year
        // or month; the others start at the counter day.
        let (month, day) = match rrule.freq {
            Frequency::Yearly => (1, 1),
            Frequency::Monthly => (c.month, 1),
            _ => (c.month, c.day),
        };
        let first_day = (|| {
            Date::new(
                i16::try_from(c.year).ok()?,
                i8::try_from(month).ok()?,
                i8::try_from(day).ok()?,
            )
            .ok()?
            // A day early, so no UTC offset can put the period's first instant before it.
            .yesterday()
            .ok()
        })();
        first_day
            .and_then(|d| add_time_to_date(self.dt_start.time_zone(), d, Time::midnight()))
            .is_some_and(|earliest| earliest > *until)
    }
}

impl Iterator for RRuleIter {
    type Item = Zoned;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.buffer.is_empty() {
            return self.buffer.pop_front();
        }

        if self.finished {
            return None;
        }

        self.finished = self.generate();

        if self.buffer.is_empty() {
            self.finished = true;
        }
        self.buffer.pop_front()
    }
}

pub(crate) trait WasLimited {
    fn was_limited(&self) -> bool;
}

impl WasLimited for RRuleIter {
    fn was_limited(&self) -> bool {
        self.was_limited
    }
}
