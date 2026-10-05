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
