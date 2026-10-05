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
