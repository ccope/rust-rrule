//! # Timezone Support
//!
//! This examples uses `RRuleSet` with one `RRule` that yields recurrences
//! in the Europe/Berlin timezone, and one EXDATE that is specified
//! in UTC and collides with one of those recurrences.

use jiff::civil::date;
use jiff::tz::TimeZone;
use rrule::{Frequency, RRule};

fn main() {
    let tz = TimeZone::get("Europe/Berlin").unwrap();
    let start_date = date(2020, 1, 1).at(9, 0, 0, 0).to_zoned(tz).unwrap();
    let exdate = date(2020, 1, 2)
        .at(8, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap();

    // Build an rrule set that occurs daily at 9:00 for 4 times
    let rrule_set = RRule::default()
        .count(4)
        .freq(Frequency::Daily)
        .build(start_date)
        .expect("RRule invalid")
        // Exdate in the UTC at 8:00 which is 9:00 in Berlin and therefore
        // collides with the second rrule occurrence.
        .exdate(exdate);

    let recurrences = rrule_set.all_unchecked();
    // RRule contained 4 recurrences but 1 was filtered away by the exdate
    assert_eq!(recurrences.len(), 3);

    // To see the occurrences in another zone, convert each `Zoned`.
    let moscow = TimeZone::get("Europe/Moscow").unwrap();
    let _recurrences_in_moscow: Vec<jiff::Zoned> = recurrences
        .iter()
        .map(|d| d.with_time_zone(moscow.clone()))
        .collect();

    println!("Done, everything worked.");
}
