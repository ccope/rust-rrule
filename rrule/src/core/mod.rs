mod datetime;
mod rrule;
mod rruleset;
pub(crate) mod utils;

pub use self::rrule::{Frequency, NWeekday, RRule};
pub use self::rruleset::{RRuleResult, RRuleSet};
pub(crate) use datetime::{
    datetime_to_ical_format, datetime_to_ical_value, floating, get_day, get_hour, get_minute,
    get_month, get_second, is_floating, is_utc, WeekdayExt,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// An empty struct to keep the validated stage
pub struct Validated;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// An empty struct to keep the unvalidated (or not-yet-validated) stage
pub struct Unvalidated;
