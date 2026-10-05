use crate::tests::common::{check_occurrences, test_recurring_rrule_set, ymd_hms};
use crate::{Frequency, NWeekday, RRule, RRuleSet, Weekday};

#[test]
#[cfg(feature = "exrule")]
fn rrule_and_exrule() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);

    let rrule1 = RRule {
        freq: Frequency::Yearly,
        count: Some(6),
        by_weekday: vec![
            NWeekday::Every(Weekday::Tuesday),
            NWeekday::Every(Weekday::Thursday),
        ],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        ..Default::default()
    };
    let rrule = rrule1.validate(dt_start.clone()).unwrap();

    let rrule2 = RRule {
        freq: Frequency::Yearly,
        count: Some(3),
        by_weekday: vec![NWeekday::Every(Weekday::Thursday)],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        ..Default::default()
    };
    let exrule = rrule2.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule).exrule(exrule);

    test_recurring_rrule_set(
        set,
        &[
            ymd_hms(1997, 9, 2, 9, 0, 0),
            ymd_hms(1997, 9, 9, 9, 0, 0),
            ymd_hms(1997, 9, 16, 9, 0, 0),
        ],
    );
}

#[test]
#[cfg(feature = "exrule")]
fn setdate_and_exdate() {
    let set = RRuleSet::new(ymd_hms(1970, 1, 1, 0, 0, 0))
        .set_rdates(vec![
            ymd_hms(1997, 9, 2, 9, 0, 0),
            ymd_hms(1997, 9, 4, 9, 0, 0),
            ymd_hms(1997, 9, 9, 9, 0, 0),
            ymd_hms(1997, 9, 11, 9, 0, 0),
            ymd_hms(1997, 9, 16, 9, 0, 0),
            ymd_hms(1997, 9, 18, 9, 0, 0),
        ])
        .set_exdates(vec![
            ymd_hms(1997, 9, 4, 9, 0, 0),
            ymd_hms(1997, 9, 11, 9, 0, 0),
            ymd_hms(1997, 9, 18, 9, 0, 0),
        ]);

    test_recurring_rrule_set(
        set,
        &[
            ymd_hms(1997, 9, 2, 9, 0, 0),
            ymd_hms(1997, 9, 9, 9, 0, 0),
            ymd_hms(1997, 9, 16, 9, 0, 0),
        ],
    );
}

#[test]
#[cfg(feature = "exrule")]
fn setdate_and_exrule() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);
    let rrule = RRule {
        freq: Frequency::Yearly,
        count: Some(3),
        by_weekday: vec![NWeekday::Every(Weekday::Thursday)],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        ..Default::default()
    };
    let exrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start)
        .set_rdates(vec![
            ymd_hms(1997, 9, 2, 9, 0, 0),
            ymd_hms(1997, 9, 4, 9, 0, 0),
            ymd_hms(1997, 9, 9, 9, 0, 0),
            ymd_hms(1997, 9, 11, 9, 0, 0),
            ymd_hms(1997, 9, 16, 9, 0, 0),
            ymd_hms(1997, 9, 18, 9, 0, 0),
        ])
        .exrule(exrule);

    test_recurring_rrule_set(
        set,
        &[
            ymd_hms(1997, 9, 2, 9, 0, 0),
            ymd_hms(1997, 9, 9, 9, 0, 0),
            ymd_hms(1997, 9, 16, 9, 0, 0),
        ],
    );
}

#[test]
fn rrule_and_exdate_1() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);
    let rrule = RRule {
        freq: Frequency::Yearly,
        count: Some(6),
        by_weekday: vec![
            NWeekday::Every(Weekday::Tuesday),
            NWeekday::Every(Weekday::Thursday),
        ],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule).set_exdates(vec![
        ymd_hms(1997, 9, 2, 9, 0, 0),
        ymd_hms(1997, 9, 4, 9, 0, 0),
        ymd_hms(1997, 9, 9, 9, 0, 0),
    ]);

    test_recurring_rrule_set(
        set,
        &[
            ymd_hms(1997, 9, 11, 9, 0, 0),
            ymd_hms(1997, 9, 16, 9, 0, 0),
            ymd_hms(1997, 9, 18, 9, 0, 0),
        ],
    );
}

#[test]
fn rrule_and_exdate_2() {
    let dates = "DTSTART;TZID=Europe/Paris:20201214T093000\n\
        RRULE:FREQ=WEEKLY;UNTIL=20210308T083000Z;INTERVAL=2;BYDAY=MO;WKST=MO\n\
        EXDATE;TZID=Europe/Paris:20201228T093000,20210125T093000,20210208T093000"
        .parse::<RRuleSet>()
        .unwrap()
        .all(u16::MAX)
        .dates;
    // This results in the following set (minus exdate)
    // [
    //     2020-12-14T09:30:00CET,
    //     2020-12-28T09:30:00CET, // Removed because of exdate
    //     2021-01-11T09:30:00CET,
    //     2021-01-25T09:30:00CET, // Removed because of exdate
    //     2021-02-08T09:30:00CET, // Removed because of exdate
    //     2021-02-22T09:30:00CET,
    //     2021-03-08T09:30:00CET, // same as `UNTIL` but different timezones
    // ]
    check_occurrences(
        &dates,
        &[
            "2020-12-14T09:30:00+01:00",
            "2021-01-11T09:30:00+01:00",
            "2021-02-22T09:30:00+01:00",
            "2021-03-08T09:30:00+01:00",
        ],
    );
}

#[test]
#[cfg(feature = "exrule")]
fn rrule_and_exyearly_yearly_big() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        count: Some(13),
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let exrule = RRule {
        freq: Frequency::Yearly,
        count: Some(10),
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let exrule = exrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule).exrule(exrule);

    test_recurring_rrule_set(
        set,
        &[
            ymd_hms(2007, 9, 2, 9, 0, 0),
            ymd_hms(2008, 9, 2, 9, 0, 0),
            ymd_hms(2009, 9, 2, 9, 0, 0),
        ],
    );
}

#[test]
#[cfg(feature = "exrule")]
fn before() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let exrule = RRule {
        freq: Frequency::Yearly,
        count: Some(10),
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let exrule = exrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start)
        .rrule(rrule)
        .exrule(exrule)
        .before(ymd_hms(2015, 9, 2, 9, 0, 0));

    assert_eq!(
        set.all_unchecked().last().unwrap().clone(),
        ymd_hms(2015, 9, 2, 9, 0, 0),
    );
}

#[test]
#[cfg(feature = "exrule")]
fn after() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let exrule = RRule {
        freq: Frequency::Yearly,
        count: Some(10),
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let exrule = exrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start)
        .rrule(rrule)
        .exrule(exrule)
        .after(ymd_hms(2000, 9, 2, 9, 0, 0));

    assert_eq!(set.all(1).dates[0], ymd_hms(2007, 9, 2, 9, 0, 0),);
}

#[test]
#[cfg(feature = "exrule")]
fn between() {
    let dt_start = ymd_hms(1997, 9, 2, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let exrule = RRule {
        freq: Frequency::Yearly,
        count: Some(10),
        by_month: vec![9],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![2],
        ..Default::default()
    };
    let exrule = exrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start)
        .rrule(rrule)
        .exrule(exrule)
        .after(ymd_hms(2000, 9, 2, 9, 0, 0))
        .before(ymd_hms(2010, 9, 2, 9, 0, 0));

    check_occurrences(
        &set.all(u16::MAX).dates,
        &[
            "2007-09-02T09:00:00-00:00",
            "2008-09-02T09:00:00-00:00",
            "2009-09-02T09:00:00-00:00",
            "2010-09-02T09:00:00-00:00",
        ],
    );
}

#[test]
fn before_70s() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        count: Some(2),
        by_month: vec![1],
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![1],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1961, 1, 1, 9, 0, 0)],
    );
}

#[test]
fn secondly_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Secondly,
        count: Some(2),
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 1, 9, 0, 1)],
    );
}

#[test]
fn secondly_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Secondly,
        count: Some(2),
        interval: 2,
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 1, 9, 0, 2)],
    );
}

#[test]
fn minutely_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Minutely,
        count: Some(2),
        by_second: vec![0],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 1, 9, 1, 0)],
    );
}

#[test]
fn minutely_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Minutely,
        count: Some(2),
        by_second: vec![0],
        interval: 2,
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 1, 9, 2, 0)],
    );
}

#[test]
fn hourly_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Hourly,
        count: Some(2),
        by_minute: vec![0],
        by_second: vec![0],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 1, 10, 0, 0)],
    );
}

#[test]
fn hourly_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Hourly,
        count: Some(2),
        by_minute: vec![0],
        by_second: vec![0],
        interval: 2,
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 1, 11, 0, 0)],
    );
}

#[test]
fn daily_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Daily,
        count: Some(2),
        by_minute: vec![0],
        by_second: vec![0],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 2, 9, 0, 0)],
    );
}

#[test]
fn daily_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Daily,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        interval: 2,
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 1, 3, 9, 0, 0)],
    );
}

#[test]
fn weekly_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 4, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Weekly,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_weekday: vec![NWeekday::Every(Weekday::Monday)],
        ..Default::default()
    };
    // 4th is Monday
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 4, 9, 0, 0), ymd_hms(1960, 1, 11, 9, 0, 0)],
    );
}

#[test]
fn weekly_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 4, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Weekly,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_weekday: vec![NWeekday::Every(Weekday::Monday)],
        interval: 2,
        ..Default::default()
    };
    // 4th is Monday
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 4, 9, 0, 0), ymd_hms(1960, 1, 18, 9, 0, 0)],
    );
}

#[test]
fn monthly_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Monthly,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_month_day: vec![1],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 2, 1, 9, 0, 0)],
    );
}

#[test]
fn monthly_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Monthly,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        interval: 2,
        by_month_day: vec![1],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1960, 3, 1, 9, 0, 0)],
    );
}

#[test]
fn yearly_with_interval_1() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_year_day: vec![1],
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1961, 1, 1, 9, 0, 0)],
    );
}

#[test]
fn yearly_with_interval_2() {
    let dt_start = ymd_hms(1960, 1, 1, 9, 0, 0);

    let rrule = RRule {
        freq: Frequency::Yearly,
        count: Some(2),
        by_hour: vec![9],
        by_minute: vec![0],
        by_second: vec![0],
        by_year_day: vec![1],
        interval: 2,
        ..Default::default()
    };
    let rrule = rrule.validate(dt_start.clone()).unwrap();

    let set = RRuleSet::new(dt_start).rrule(rrule);

    test_recurring_rrule_set(
        set,
        &[ymd_hms(1960, 1, 1, 9, 0, 0), ymd_hms(1962, 1, 1, 9, 0, 0)],
    );
}

// Upstream fmeringdal/rust-rrule#150: RFC 5545 §3.8.5.2 defines the recurrence set as a
// set, so an instant produced twice is still one occurrence.
#[test]
fn rdate_equal_to_a_rule_occurrence_is_emitted_once() {
    let dates = "DTSTART:20240101T090000Z\n\
        RRULE:FREQ=DAILY;COUNT=2\n\
        RDATE:20240102T090000Z"
        .parse::<RRuleSet>()
        .unwrap()
        .all(10)
        .dates;
    check_occurrences(
        &dates,
        &["2024-01-01T09:00:00+00:00", "2024-01-02T09:00:00+00:00"],
    );
}

#[test]
fn two_rules_producing_the_same_instant_emit_it_once() {
    let dates = "DTSTART:20240101T090000Z\n\
        RRULE:FREQ=DAILY;COUNT=3\n\
        RRULE:FREQ=DAILY;INTERVAL=2;COUNT=2"
        .parse::<RRuleSet>()
        .unwrap()
        .all(10)
        .dates;
    check_occurrences(
        &dates,
        &[
            "2024-01-01T09:00:00+00:00",
            "2024-01-02T09:00:00+00:00",
            "2024-01-03T09:00:00+00:00",
        ],
    );
}

// Upstream fmeringdal/rust-rrule#146: an all-day series (DATE DTSTART) excludes days
// with EXDATE;VALUE=DATE, which is how Google Calendar writes them.
#[test]
fn all_day_series_honours_date_valued_exdates() {
    let set: RRuleSet = "DTSTART;VALUE=DATE:20260401\n\
        RRULE:FREQ=DAILY;COUNT=5\n\
        EXDATE;VALUE=DATE:20260402,20260404"
        .parse()
        .unwrap();
    let days: Vec<String> = set
        .all(10)
        .dates
        .iter()
        .map(|d| d.date().to_string())
        .collect();
    assert_eq!(days, ["2026-04-01", "2026-04-03", "2026-04-05"]);
}

// A DATE DTSTART is written back as a DATE, with its UNTIL, RDATEs and EXDATEs, and
// without the midnight BYHOUR/BYMINUTE/BYSECOND that RFC 5545 forbids beside it.
#[test]
fn a_date_start_is_written_back_as_dates() {
    let text = "DTSTART;VALUE=DATE:20270501\n\
        RRULE:FREQ=DAILY;UNTIL=20270510\n\
        RDATE;VALUE=DATE:20270601\n\
        EXDATE;VALUE=DATE:20270503";
    let set = text.parse::<RRuleSet>().unwrap();
    assert_eq!(set.to_string(), text);
    assert_eq!(
        set.to_string().parse::<RRuleSet>().unwrap().all(20).dates,
        set.all(20).dates
    );
}

#[test]
fn a_bare_date_start_is_written_back_as_a_date() {
    let set = "DTSTART:20270501\nRRULE:FREQ=WEEKLY;COUNT=2"
        .parse::<RRuleSet>()
        .unwrap();
    assert_eq!(
        set.to_string(),
        "DTSTART;VALUE=DATE:20270501\nRRULE:FREQ=WEEKLY;COUNT=2;BYDAY=SA"
    );
}

// Applied by default, so dropping it would change the occurrences.
#[test]
fn a_date_start_keeps_a_byhour_other_than_midnight_when_written_back() {
    let set = "DTSTART;VALUE=DATE:20270501\nRRULE:FREQ=DAILY;COUNT=2;BYHOUR=9"
        .parse::<RRuleSet>()
        .unwrap();
    assert_eq!(
        set.to_string(),
        "DTSTART;VALUE=DATE:20270501\nRRULE:FREQ=DAILY;COUNT=2;BYHOUR=9"
    );
}

#[test]
fn a_date_time_start_at_midnight_is_still_written_back_as_a_date_time() {
    let set = "DTSTART:20270501T000000\nRRULE:FREQ=DAILY;COUNT=2"
        .parse::<RRuleSet>()
        .unwrap();
    assert_eq!(
        set.to_string(),
        "DTSTART:20270501T000000\nRRULE:FREQ=DAILY;COUNT=2;BYHOUR=0;BYMINUTE=0;BYSECOND=0"
    );
}
