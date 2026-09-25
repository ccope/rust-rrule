use crate::tests::common;
use crate::{Frequency, RRule, RRuleSet, Unvalidated, Weekday};

#[test]
fn issue_34() {
    let dates = "DTSTART;TZID=America/New_York:19970929T090000
RRULE:FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-2"
        .parse::<RRuleSet>()
        .unwrap()
        .all(7)
        .dates;
    common::check_occurrences(
        &dates,
        &[
            "1997-09-29T09:00:00-04:00",
            "1997-10-30T09:00:00-05:00",
            "1997-11-27T09:00:00-05:00",
            "1997-12-30T09:00:00-05:00",
            "1998-01-29T09:00:00-05:00",
            "1998-02-26T09:00:00-05:00",
            "1998-03-30T09:00:00-05:00",
        ],
    );
}

#[test]
fn issue_49() {
    let rrule_set = "DTSTART:20211214T091500\nEXDATE:20211228T091500,20220104T091500\nRRULE:FREQ=WEEKLY;UNTIL=20220906T091500;INTERVAL=1;BYDAY=TU;WKST=MO"
        .parse::<RRuleSet>()
        .expect("The RRule is not valid");

    let res = rrule_set.all(1).dates;
    assert!(!res.is_empty());
    let res_str = format!("{}", res[0]);
    // Check that result datetime is not in UTC
    assert!(!res_str.contains("UTC"));
}

#[test]
fn issue_61() {
    let rrule_set = "DTSTART;TZID=Europe/Berlin:18930401T010000\nRRULE:FREQ=DAILY"
        .parse::<RRuleSet>()
        .expect("The RRule is not valid");

    let res = rrule_set.all(10).dates;
    assert_eq!(res.len(), 10);
}

// Frequency should be capitalized
#[test]
fn issue_97() {
    let rrule = RRule::new(Frequency::Yearly)
        .by_month_day((24..=26).collect())
        .by_month(&[12]);

    assert_eq!(
        rrule.to_string(),
        "FREQ=YEARLY;BYMONTH=12;BYMONTHDAY=24,25,26"
    );
}

#[test]
fn issue_111() {
    let rrule = "RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=TU;WKST=SU".parse::<RRule<Unvalidated>>();

    // Convert to string...
    let rrule_str = format!("{}", rrule.unwrap());
    assert!(rrule_str.contains("WKST=SU"));
}

// A weekday is sliced two *bytes* from the end, so a multi-byte character there
// panicked instead of failing to parse.
#[test]
fn byday_with_multibyte_characters_is_a_parse_error() {
    for byday in ["1€", "€", "M€", "€MO", "1é", "ÉMO"] {
        let text = format!("DTSTART:20200101T090000Z\nRRULE:FREQ=WEEKLY;BYDAY={byday}");
        assert!(
            text.parse::<RRuleSet>().is_err(),
            "BYDAY={byday} should not parse"
        );
    }
}

// Jiff's last instant is 9999-12-30T22:00Z; a weekly iterator starting late in 9999
// builds dates past it and must stop rather than panic.
#[test]
fn dtstart_at_the_end_of_the_year_range_is_rejected() {
    let text = "DTSTART:99991225T000000Z\nRRULE:FREQ=WEEKLY;COUNT=3";
    assert!(text.parse::<RRuleSet>().is_err());
}

#[test]
fn iterating_near_the_ends_of_the_supported_range_does_not_panic() {
    use jiff::tz::TimeZone;
    let validated_at = common::ymd_hms(2020, 1, 1, 0, 0, 0);
    let mut rules = vec![RRule::new(Frequency::Weekly).count(3)];
    // BYWEEKNO looks at the previous year's weeks for some week starts only.
    for wkst in [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ] {
        rules.push(
            RRule::new(Frequency::Yearly)
                .count(3)
                .by_week_no(vec![1, 53])
                .week_start(wkst),
        );
    }
    for rule in rules {
        let rule = rule.validate(validated_at.clone()).unwrap();
        for start in [jiff::Timestamp::MAX, jiff::Timestamp::MIN] {
            // The rule was validated against another DTSTART, which the API allows.
            let set = RRuleSet::new(start.to_zoned(TimeZone::UTC)).rrule(rule.clone());
            let _ = set.all(10);
        }
    }
}

// An ordinal that is not a number used to be read as 0, i.e. "every", so
// `33331TU` quietly meant every Tuesday.
#[test]
fn byday_with_a_malformed_ordinal_is_a_parse_error() {
    for byday in ["-.MO", "2S=SU", "1MOL1TU", "33331TU", "1-MO", "--1MO"] {
        let text = format!("DTSTART:20200101T090000Z\nRRULE:FREQ=MONTHLY;BYDAY={byday}");
        assert!(
            text.parse::<RRuleSet>().is_err(),
            "BYDAY={byday} should not parse"
        );
    }
    for byday in ["MO", "1MO", "+2TU", "-1FR", "5SU"] {
        let text = format!("DTSTART:20200101T090000Z\nRRULE:FREQ=MONTHLY;BYDAY={byday}");
        assert!(
            text.parse::<RRuleSet>().is_ok(),
            "BYDAY={byday} should parse"
        );
    }
}

// Found by AFL: a rule that stops matching (there is no 9th Sunday in April) ran on
// to the iteration guard, far past its UNTIL, because UNTIL was only checked
// against generated occurrences.
#[test]
fn until_ends_iteration_even_when_nothing_matches() {
    let set = "DTSTART:20060416\nRRULE:FREQ=YEARLY;UNTIL=20070405;BYDAY=9SU;BYMONTH=4"
        .parse::<RRuleSet>()
        .unwrap();
    let started = std::time::Instant::now();
    assert!(set.all(50).dates.is_empty());
    // Without the UNTIL check this walks ~8000 years to the end of the year range,
    // taking seconds in a debug build; with it, one year.
    let elapsed = started.elapsed();
    assert!(elapsed.as_millis() < 250, "took {elapsed:?}");
}

#[test]
fn dtstart_before_year_1_is_rejected() {
    let rule = "RRULE:FREQ=YEARLY;COUNT=2";
    assert!(format!("DTSTART:00001231T000000Z\n{rule}")
        .parse::<RRuleSet>()
        .is_err());
    assert!(format!("DTSTART:00010101T000000Z\n{rule}")
        .parse::<RRuleSet>()
        .is_ok());
}

// RRuleResult::limited says the result may be incomplete; a rule that never
// matches runs into the iteration guard, which has to show up there.
#[test]
fn hitting_the_iteration_guard_is_reported_as_limited() {
    let result = "DTSTART:20200101T090000Z\nRRULE:FREQ=DAILY;BYMONTH=2;BYMONTHDAY=30"
        .parse::<RRuleSet>()
        .unwrap()
        .all(10);
    assert!(result.dates.is_empty());
    assert!(result.limited);
}
