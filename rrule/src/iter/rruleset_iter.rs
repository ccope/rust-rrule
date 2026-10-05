use jiff::Zoned;

use super::rrule_iter::RRuleIter;
use super::rrule_iter::WasLimited;
use crate::core::{get_hour, get_minute, get_month, get_second};
use crate::RRuleError;
use crate::RRuleSet;
use crate::{Frequency, RRule};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::{collections::HashMap, iter::Iterator};

#[derive(Debug, Clone)]
/// Iterator over all the dates in an [`RRuleSet`].
pub struct RRuleSetIter {
    queue: HashMap<usize, Zoned>,
    step_limit: Option<u32>,
    rrule_iters: Vec<RRuleIter>,
    exrules: Vec<RRuleIter>,
    exdates: BTreeSet<i64>,
    /// Sorted additional dates in descending order
    rdates: Vec<Zoned>,
    was_limited: bool,
    /// The instant last returned, so one produced twice is returned once (RFC 5545 §3.8.5.2).
    last: Option<jiff::Timestamp>,
}

impl RRuleSetIter {
    fn generate_date(
        dates: &mut Vec<Zoned>,
        exrules: &mut [RRuleIter],
        exdates: &mut BTreeSet<i64>,
        step_limit: Option<u32>,
    ) -> (Option<Zoned>, bool) {
        if dates.is_empty() {
            return (None, false);
        }

        let mut date = dates.remove(dates.len() - 1);
        let mut loop_counter: u32 = 0;
        while Self::is_date_excluded(&date, exrules, exdates) {
            if dates.is_empty() {
                return (None, false);
            }
            // Prevent infinite loops
            if let Some(step_limit) = step_limit {
                loop_counter += 1;
                if loop_counter >= step_limit {
                    log::warn!(
                        "Reached max loop counter (`{}`). \
                See 'validator limits' in docs for more info.",
                        step_limit
                    );
                    return (None, true);
                }
            }
            date = dates.remove(dates.len() - 1);
        }

        (Some(date), false)
    }

    fn generate(
        rrule_iter: &mut RRuleIter,
        exrules: &mut [RRuleIter],
        exdates: &mut BTreeSet<i64>,
        step_limit: Option<u32>,
    ) -> (Option<Zoned>, bool) {
        let mut date = match rrule_iter.next() {
            Some(d) => d,
            None => return (None, false),
        };
        let mut loop_counter: u32 = 0;
        while Self::is_date_excluded(&date, exrules, exdates) {
            // Prevent infinite loops
            if let Some(step_limit) = step_limit {
                loop_counter += 1;
                if loop_counter >= step_limit {
                    log::warn!(
                        "Reached max loop counter (`{}`). \
                    See 'validator limits' in docs for more info.",
                        step_limit
                    );
                    return (None, true);
                }
            }

            date = match rrule_iter.next() {
                Some(d) => d,
                None => return (None, false),
            };
        }

        (Some(date), false)
    }

    fn is_date_excluded(
        date: &Zoned,
        exrules: &mut [RRuleIter],
        exdates: &mut BTreeSet<i64>,
    ) -> bool {
        for exrule in exrules {
            for exdate in exrule {
                exdates.insert(exdate.timestamp().as_second());
                if exdate > *date {
                    break;
                }
            }
        }

        exdates.contains(&date.timestamp().as_second())
    }
}

impl Iterator for RRuleSetIter {
    type Item = Zoned;

    fn next(&mut self) -> Option<Self::Item> {
        // Candidates arrive in ascending order, so a duplicate is always the previous one.
        loop {
            let date = self.next_candidate()?;
            let instant = date.timestamp();
            if self.last != Some(instant) {
                self.last = Some(instant);
                return Some(date);
            }
        }
    }
}

impl RRuleSetIter {
    fn next_candidate(&mut self) -> Option<Zoned> {
        let mut next_date: Option<(usize, Zoned)> = None;

        // If there already was an error, return the error again.
        if self.was_limited {
            return None;
        }

        for (i, rrule_iter) in self.rrule_iters.iter_mut().enumerate() {
            let rrule_queue = self.queue.remove(&i);
            let next_rrule_date = if let Some(d) = rrule_queue {
                Some(d)
            } else {
                // should be method on self
                let (date, was_limited) = Self::generate(
                    rrule_iter,
                    &mut self.exrules,
                    &mut self.exdates,
                    self.step_limit,
                );

                if was_limited {
                    self.was_limited = true;
                    return None;
                }

                date
            };

            if let Some(next_rrule_date) = next_rrule_date {
                match next_date.take() {
                    None => next_date = Some((i, next_rrule_date)),
                    Some((idx, date)) => {
                        if date >= next_rrule_date {
                            // Add previous date to its rrule queue
                            self.queue.insert(idx, date);

                            // Update next_date
                            next_date = Some((i, next_rrule_date));
                        } else {
                            // Store for next iterations
                            self.queue.insert(i, next_rrule_date);
                            next_date = Some((idx, date));
                        }
                    }
                }
            }
        }

        let (generated_date, was_limited) = Self::generate_date(
            &mut self.rdates,
            &mut self.exrules,
            &mut self.exdates,
            self.step_limit,
        );
        if was_limited {
            self.was_limited = true;
            return None;
        }

        match generated_date {
            Some(first_rdate) => {
                let next_date = match next_date {
                    Some(next_date) => {
                        if next_date.1 >= first_rdate {
                            // Add previous date to its rrule queue
                            self.queue.insert(next_date.0, next_date.1);

                            first_rdate
                        } else {
                            // add rdate back
                            self.rdates.push(first_rdate);

                            next_date.1
                        }
                    }
                    None => first_rdate,
                };
                Some(next_date)
            }
            None => next_date.map(|d| d.1),
        }
    }
}

impl IntoIterator for &RRuleSet {
    type Item = Zoned;

    type IntoIter = RRuleSetIter;

    fn into_iter(self) -> Self::IntoIter {
        // Sort in decreasing order
        let mut rdates_sorted = self.rdate.clone();
        // An implicit RDATE goes through EXDATE and EXRULE like any other, and the
        // iterator's deduplication drops it when a rule generates it too.
        if self.dtstart_always_occurs {
            rdates_sorted.push(self.dt_start.clone());
        }
        rdates_sorted
            .sort_by(|d1, d2| d2.partial_cmp(d1).expect("Could not order dates correctly"));

        let step_limit = self.limited.then_some(self.iteration_limit);
        let iter = |rule: &RRule| reading(self, rule).iter_with_ctx(&self.dt_start, step_limit);

        RRuleSetIter {
            queue: HashMap::new(),
            step_limit,
            rrule_iters: self.rrule.iter().map(iter).collect(),
            rdates: rdates_sorted,
            exrules: self.exrule.iter().map(iter).collect(),
            exdates: self
                .exdate
                .iter()
                .map(|exdate| exdate.timestamp().as_second())
                .collect(),
            was_limited: false,
            last: None,
        }
    }
}

/// The rule as the set's readings have it iterated; the stored rule, and so what
/// the set writes back, is unchanged.
fn reading<'a>(set: &RRuleSet, rule: &'a RRule) -> Cow<'a, RRule> {
    let mut rule = Cow::Borrowed(rule);
    // See RRuleSet::yearly_bymonthday_uses_dtstart_month.
    if set.yearly_bymonthday_uses_dtstart_month
        && rule.freq == Frequency::Yearly
        && rule.by_month.is_empty()
        && (!rule.by_month_day.is_empty() || !rule.by_n_month_day.is_empty())
        && rule.by_week_no.is_empty()
        && rule.by_year_day.is_empty()
    {
        rule.to_mut().by_month = vec![get_month(&set.dt_start)];
    }
    // See RRuleSet::date_start_ignores_time_parts.
    if set.date_start_ignores_time_parts && set.dt_start_is_date && rule.freq < Frequency::Hourly {
        let rule = rule.to_mut();
        rule.by_hour = vec![get_hour(&set.dt_start)];
        rule.by_minute = vec![get_minute(&set.dt_start)];
        rule.by_second = vec![get_second(&set.dt_start)];
    }
    rule
}

impl WasLimited for RRuleSetIter {
    fn was_limited(&self) -> bool {
        // A rule iterator that hit the guard just stops yielding, so its flag is
        // the only record that the set's dates may be incomplete.
        self.was_limited
            || self.rrule_iters.iter().any(WasLimited::was_limited)
            || self.exrules.iter().any(WasLimited::was_limited)
    }
}

impl FromStr for RRuleSetIter {
    type Err = RRuleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RRuleSet::from_str(s)?.into_iter())
    }
}
