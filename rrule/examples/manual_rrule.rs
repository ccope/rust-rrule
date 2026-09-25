//! # Manual [`RRule`]
//!
//! Create an [`RRule`] object.

use jiff::civil::date;
use jiff::tz::TimeZone;
use rrule::{Frequency, RRule};

fn main() {
    // Build an RRuleSet that starts the first day in 2020 at 9:00AM and occurs daily 5 times
    let start_date = date(2020, 1, 1)
        .at(9, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap();
    let rrule_set = RRule::default()
        .count(5)
        .freq(Frequency::Daily)
        .build(start_date)
        .expect("RRule invalid");

    let recurrences = rrule_set.all_unchecked();
    for (i, rec) in recurrences.iter().enumerate() {
        assert_eq!(rec.year(), 2020);
        assert_eq!(rec.month(), 1);
        assert_eq!(rec.day(), 1 + i as i8);
        assert_eq!(rec.hour(), 9);
    }
    assert_eq!(recurrences.len(), 5);
    println!("Done, everything worked.");
}
