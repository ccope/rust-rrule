use crate::tests::common;
use crate::RRuleSet;

// Wednesday 6 January 2027; the rule only generates Mondays.
const OFF_RULE_DTSTART: &str = "DTSTART:20270106T090000Z\nRRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=2";

#[test]
fn an_off_rule_dtstart_is_not_an_occurrence_by_default() {
    let dates = OFF_RULE_DTSTART.parse::<RRuleSet>().unwrap().all(10).dates;
    common::check_occurrences(
        &dates,
        &["2027-01-11T09:00:00+00:00", "2027-01-18T09:00:00+00:00"],
    );
}

#[test]
fn dtstart_always_occurs_adds_an_off_rule_dtstart_outside_count() {
    let dates = OFF_RULE_DTSTART
        .parse::<RRuleSet>()
        .unwrap()
        .dtstart_always_occurs(true)
        .all(10)
        .dates;
    common::check_occurrences(
        &dates,
        &[
            "2027-01-06T09:00:00+00:00",
            "2027-01-11T09:00:00+00:00",
            "2027-01-18T09:00:00+00:00",
        ],
    );
}

#[test]
fn an_exdate_removes_an_off_rule_dtstart_that_always_occurs() {
    let dates = format!("{OFF_RULE_DTSTART}\nEXDATE:20270106T090000Z")
        .parse::<RRuleSet>()
        .unwrap()
        .dtstart_always_occurs(true)
        .all(10)
        .dates;
    common::check_occurrences(
        &dates,
        &["2027-01-11T09:00:00+00:00", "2027-01-18T09:00:00+00:00"],
    );
}

#[test]
fn dtstart_always_occurs_does_not_repeat_an_on_rule_dtstart() {
    let dates = "DTSTART:20270104T090000Z\nRRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=2"
        .parse::<RRuleSet>()
        .unwrap()
        .dtstart_always_occurs(true)
        .all(10)
        .dates;
    common::check_occurrences(
        &dates,
        &["2027-01-04T09:00:00+00:00", "2027-01-11T09:00:00+00:00"],
    );
}

fn yearly_pinned(rule: &str) -> Vec<jiff::Zoned> {
    rule.parse::<RRuleSet>()
        .unwrap()
        .yearly_bymonthday_uses_dtstart_month(true)
        .all(10)
        .dates
}

#[test]
fn yearly_bymonthday_without_bymonth_repeats_in_every_month_by_default() {
    let dates = "DTSTART:20270215T090000Z\nRRULE:FREQ=YEARLY;BYMONTHDAY=15;COUNT=3"
        .parse::<RRuleSet>()
        .unwrap()
        .all(10)
        .dates;
    common::check_occurrences(
        &dates,
        &[
            "2027-02-15T09:00:00+00:00",
            "2027-03-15T09:00:00+00:00",
            "2027-04-15T09:00:00+00:00",
        ],
    );
}

#[test]
fn yearly_bymonthday_uses_dtstart_month_when_bymonth_is_missing() {
    let dates = yearly_pinned("DTSTART:20270215T090000Z\nRRULE:FREQ=YEARLY;BYMONTHDAY=15;COUNT=3");
    common::check_occurrences(
        &dates,
        &[
            "2027-02-15T09:00:00+00:00",
            "2028-02-15T09:00:00+00:00",
            "2029-02-15T09:00:00+00:00",
        ],
    );
}

#[test]
fn yearly_bymonthday_uses_dtstart_month_for_a_negative_day() {
    let dates = yearly_pinned("DTSTART:20270331T090000Z\nRRULE:FREQ=YEARLY;BYMONTHDAY=-1;COUNT=2");
    common::check_occurrences(
        &dates,
        &["2027-03-31T09:00:00+00:00", "2028-03-31T09:00:00+00:00"],
    );
}

// Google pins the month with BYDAY present too: Friday the 13th of August only.
#[test]
fn yearly_bymonthday_uses_dtstart_month_alongside_byday() {
    let dates =
        yearly_pinned("DTSTART:20270813T090000Z\nRRULE:FREQ=YEARLY;BYMONTHDAY=13;BYDAY=FR;COUNT=2");
    common::check_occurrences(
        &dates,
        &["2027-08-13T09:00:00+00:00", "2032-08-13T09:00:00+00:00"],
    );
}

#[test]
fn yearly_byday_without_bymonthday_is_not_pinned_to_dtstart_month() {
    let dates = yearly_pinned("DTSTART:20270104T090000Z\nRRULE:FREQ=YEARLY;BYDAY=MO;COUNT=5");
    common::check_occurrences(
        &dates,
        &[
            "2027-01-04T09:00:00+00:00",
            "2027-01-11T09:00:00+00:00",
            "2027-01-18T09:00:00+00:00",
            "2027-01-25T09:00:00+00:00",
            "2027-02-01T09:00:00+00:00",
        ],
    );
}

#[test]
fn yearly_bymonthday_keeps_an_explicit_bymonth() {
    let dates = yearly_pinned(
        "DTSTART:20270615T090000Z\nRRULE:FREQ=YEARLY;BYMONTH=6,7;BYMONTHDAY=15;COUNT=2",
    );
    common::check_occurrences(
        &dates,
        &["2027-06-15T09:00:00+00:00", "2027-07-15T09:00:00+00:00"],
    );
}

#[test]
fn yearly_bymonthday_uses_dtstart_month_without_writing_bymonth() {
    let text = "DTSTART:20270215T090000Z\nRRULE:FREQ=YEARLY;COUNT=3;BYMONTHDAY=15";
    let set = text
        .parse::<RRuleSet>()
        .unwrap()
        .yearly_bymonthday_uses_dtstart_month(true);
    assert!(!set.to_string().contains("BYMONTH="), "{set}");
}

// DTSTART 10 February is off-rule, and the 15th stays in February.
#[test]
fn google_compat_turns_on_both_readings() {
    let dates = "DTSTART:20270210T090000Z\nRRULE:FREQ=YEARLY;BYMONTHDAY=15;COUNT=2"
        .parse::<RRuleSet>()
        .unwrap()
        .google_compat()
        .all(10)
        .dates;
    common::check_occurrences(
        &dates,
        &[
            "2027-02-10T09:00:00+00:00",
            "2027-02-15T09:00:00+00:00",
            "2028-02-15T09:00:00+00:00",
        ],
    );
}

// RFC 5545 forbids BYHOUR, BYMINUTE and BYSECOND with a DATE DTSTART; Google ignores them.
const DATE_START_BYHOUR: &str = "DTSTART;VALUE=DATE:20270501\nRRULE:FREQ=DAILY;BYHOUR=9;COUNT=2";

fn hours(dates: &[jiff::Zoned]) -> Vec<(jiff::civil::Date, i8)> {
    dates.iter().map(|z| (z.date(), z.hour())).collect()
}

#[test]
fn time_parts_on_a_date_start_apply_by_default() {
    let dates = DATE_START_BYHOUR.parse::<RRuleSet>().unwrap().all(10).dates;
    assert_eq!(
        hours(&dates),
        [
            (jiff::civil::date(2027, 5, 1), 9),
            (jiff::civil::date(2027, 5, 2), 9)
        ]
    );
}

#[test]
fn date_start_ignores_time_parts_keeps_occurrences_at_midnight() {
    let dates = DATE_START_BYHOUR
        .parse::<RRuleSet>()
        .unwrap()
        .date_start_ignores_time_parts(true)
        .all(10)
        .dates;
    assert_eq!(
        hours(&dates),
        [
            (jiff::civil::date(2027, 5, 1), 0),
            (jiff::civil::date(2027, 5, 2), 0)
        ]
    );
}

#[test]
fn date_start_ignores_time_parts_leaves_a_date_time_start_alone() {
    let dates = "DTSTART:20270501T000000Z\nRRULE:FREQ=DAILY;BYHOUR=9;COUNT=1"
        .parse::<RRuleSet>()
        .unwrap()
        .date_start_ignores_time_parts(true)
        .all(10)
        .dates;
    common::check_occurrences(&dates, &["2027-05-01T09:00:00+00:00"]);
}

// With BYHOUR applied, the rule's 09:00 and the implicit DTSTART at midnight were
// two instances on the first day.
#[test]
fn google_compat_gives_a_date_start_with_byhour_one_instance_a_day() {
    let dates = DATE_START_BYHOUR
        .parse::<RRuleSet>()
        .unwrap()
        .google_compat()
        .all(10)
        .dates;
    assert_eq!(
        hours(&dates),
        [
            (jiff::civil::date(2027, 5, 1), 0),
            (jiff::civil::date(2027, 5, 2), 0)
        ]
    );
}
