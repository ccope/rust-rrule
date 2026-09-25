use crate::{tests::common::check_occurrences, RRuleSet};

#[test]
fn daylight_savings_1() {
    let rrule: RRuleSet =
        "DTSTART;TZID=America/Vancouver:20210301T022210\nRRULE:FREQ=DAILY;COUNT=30"
            .parse()
            .unwrap();

    let dates = rrule.all_unchecked();
    check_occurrences(
        &dates,
        &[
            "2021-03-01T02:22:10-08:00",
            "2021-03-02T02:22:10-08:00",
            "2021-03-03T02:22:10-08:00",
            "2021-03-04T02:22:10-08:00",
            "2021-03-05T02:22:10-08:00",
            "2021-03-06T02:22:10-08:00",
            "2021-03-07T02:22:10-08:00",
            "2021-03-08T02:22:10-08:00",
            "2021-03-09T02:22:10-08:00",
            "2021-03-10T02:22:10-08:00",
            "2021-03-11T02:22:10-08:00",
            "2021-03-12T02:22:10-08:00",
            "2021-03-13T02:22:10-08:00",
            "2021-03-14T03:22:10-07:00",
            "2021-03-15T02:22:10-07:00",
            "2021-03-16T02:22:10-07:00",
            "2021-03-17T02:22:10-07:00",
            "2021-03-18T02:22:10-07:00",
            "2021-03-19T02:22:10-07:00",
            "2021-03-20T02:22:10-07:00",
            "2021-03-21T02:22:10-07:00",
            "2021-03-22T02:22:10-07:00",
            "2021-03-23T02:22:10-07:00",
            "2021-03-24T02:22:10-07:00",
            "2021-03-25T02:22:10-07:00",
            "2021-03-26T02:22:10-07:00",
            "2021-03-27T02:22:10-07:00",
            "2021-03-28T02:22:10-07:00",
            "2021-03-29T02:22:10-07:00",
            "2021-03-30T02:22:10-07:00",
        ],
    );
}

#[test]
fn daylight_savings_2() {
    let dates = "DTSTART;TZID=Europe/Paris:20210214T093000\n\
        RRULE:FREQ=WEEKLY;UNTIL=20210508T083000Z;INTERVAL=2;BYDAY=MO;WKST=MO"
        .parse::<RRuleSet>()
        .unwrap()
        .all(u16::MAX)
        .dates;
    check_occurrences(
        &dates,
        &[
            "2021-02-22T09:30:00+01:00",
            "2021-03-08T09:30:00+01:00",
            "2021-03-22T09:30:00+01:00",
            "2021-04-05T09:30:00+02:00", // Switching to daylight saving time.
            "2021-04-19T09:30:00+02:00",
            "2021-05-03T09:30:00+02:00",
        ],
    );
}

#[test]
fn repeated_local_time_resolves_to_first_occurrence() {
    // 01:30 happens twice on 2021-11-07 in Vancouver; RFC 5545 §3.3.5 picks the first (PDT).
    let dates = "DTSTART;TZID=America/Vancouver:20211106T013000\nRRULE:FREQ=DAILY;COUNT=3"
        .parse::<RRuleSet>()
        .unwrap()
        .all(10)
        .dates;
    check_occurrences(
        &dates,
        &[
            "2021-11-06T01:30:00-07:00",
            "2021-11-07T01:30:00-07:00",
            "2021-11-08T01:30:00-08:00",
        ],
    );
}

#[test]
fn dtstart_inside_a_gap_uses_the_offset_before_it() {
    // 02:30 does not exist on 2021-03-14 in Vancouver; it resolves one hour later, as PDT.
    // Whether later occurrences should be at 02:30 or at the resolved 03:30 is left
    // to recorded Google fixtures; only the DTSTART itself is pinned here.
    let dates = "DTSTART;TZID=America/Vancouver:20210314T023000\nRRULE:FREQ=DAILY;COUNT=1"
        .parse::<RRuleSet>()
        .unwrap()
        .all(10)
        .dates;
    check_occurrences(&dates, &["2021-03-14T03:30:00-07:00"]);
}

#[test]
fn floating_occurrences_keep_their_wall_clock_time() {
    // A floating 02:30 never meets a DST transition, wherever the code runs.
    let set: RRuleSet = "DTSTART:20210313T023000\nRRULE:FREQ=DAILY;COUNT=3"
        .parse()
        .unwrap();
    let dates = set.clone().all(10).dates;
    let times: Vec<String> = dates
        .iter()
        .map(|d| d.datetime().strftime("%Y-%m-%dT%H:%M:%S").to_string())
        .collect();
    assert_eq!(
        times,
        [
            "2021-03-13T02:30:00",
            "2021-03-14T02:30:00",
            "2021-03-15T02:30:00"
        ]
    );
    assert!(dates.iter().all(|d| d.time_zone().is_unknown()));
    assert_eq!(
        set.to_string(),
        "DTSTART:20210313T023000\nRRULE:FREQ=DAILY;COUNT=3;BYHOUR=2;BYMINUTE=30;BYSECOND=0"
    );
}
