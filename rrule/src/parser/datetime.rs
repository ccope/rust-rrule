use super::{regex::ParsedDateString, ParseError};
use crate::core::floating;
use crate::NWeekday;
use jiff::civil::{DateTime, Weekday};
use jiff::tz::TimeZone;
use jiff::Zoned;

/// Looks up an IANA time zone name, as used in a `TZID` parameter.
pub(crate) fn parse_timezone(tz: &str) -> Result<TimeZone, ParseError> {
    TimeZone::get(tz).map_err(|_| ParseError::InvalidTimezone(tz.into()))
}

/// Parses an iCalendar DATE or DATE-TIME value.
///
/// A value ending in `Z` is UTC. Otherwise it is local time in `tz`, or
/// floating when there is no `tz`. A DATE is taken as midnight. A local time
/// the zone skips or repeats is resolved as RFC 5545 §3.3.5 requires: a
/// skipped time by the offset before the gap, a repeated one by its first
/// occurrence.
pub(crate) fn datestring_to_date(
    dt: &str,
    tz: Option<&TimeZone>,
    property: &str,
) -> Result<Zoned, ParseError> {
    let invalid = || ParseError::InvalidDateTime {
        value: dt.into(),
        property: property.into(),
    };
    let ParsedDateString {
        year,
        month,
        day,
        time,
        flags,
    } = ParsedDateString::from_ical_datetime(dt).map_err(|_| invalid())?;

    let (hour, min, sec) = time.map_or((0, 0, 0), |time| (time.hour, time.min, time.sec));
    let datetime = DateTime::new(
        i16::try_from(year).map_err(|_| invalid())?,
        i8::try_from(month).map_err(|_| invalid())?,
        i8::try_from(day).map_err(|_| invalid())?,
        i8::try_from(hour).map_err(|_| invalid())?,
        i8::try_from(min).map_err(|_| invalid())?,
        i8::try_from(sec).map_err(|_| invalid())?,
        0,
    )
    .map_err(|_| invalid())?;

    let zone = if flags.zulu_timezone_set {
        TimeZone::UTC
    } else {
        tz.cloned().unwrap_or_else(floating)
    };
    zone.to_zoned(datetime).map_err(|_| invalid())
}

/// Attempts to convert a `str` to a `Weekday`.
pub(crate) fn str_to_weekday(d: &str) -> Result<Weekday, ParseError> {
    let day = match &d.to_uppercase()[..] {
        "MO" => Weekday::Monday,
        "TU" => Weekday::Tuesday,
        "WE" => Weekday::Wednesday,
        "TH" => Weekday::Thursday,
        "FR" => Weekday::Friday,
        "SA" => Weekday::Saturday,
        "SU" => Weekday::Sunday,
        _ => return Err(ParseError::InvalidWeekday(d.to_string())),
    };
    Ok(day)
}

/// Parse the "BYWEEKDAY" and "BYDAY" values
/// Example: `SU,MO,TU,WE,TH,FR` or `4MO` or `-1WE`
/// > For example, within a MONTHLY rule, +1MO (or simply 1MO) represents the first Monday
/// > within the month, whereas -1MO represents the last Monday of the month.
pub(crate) fn parse_weekdays(val: &str) -> Result<Vec<NWeekday>, ParseError> {
    let mut wdays = vec![];
    // Separate all days
    for day in val.split(',') {
        let wday = day.parse::<NWeekday>()?;
        wdays.push(wday);
    }
    Ok(wdays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::compat::Tz;

    // Canonical name: the legacy `US/Pacific` alias is missing from hosts
    // that ship tzdata without its backward links.
    const US_PACIFIC: Tz = Tz::America__Los_Angeles;

    #[test]
    fn parses_valid_nweekdays() {
        let tests = [
            ("SU", vec![NWeekday::Every(Weekday::Sunday)]),
            ("-12TU", vec![NWeekday::Nth(-12, Weekday::Tuesday)]),
            (
                "MO,WE",
                vec![
                    NWeekday::Every(Weekday::Monday),
                    NWeekday::Every(Weekday::Wednesday),
                ],
            ),
            (
                "MO,WE,3TU,-4SA",
                vec![
                    NWeekday::Every(Weekday::Monday),
                    NWeekday::Every(Weekday::Wednesday),
                    NWeekday::Nth(3, Weekday::Tuesday),
                    NWeekday::Nth(-4, Weekday::Saturday),
                ],
            ),
        ];

        for (input, expected_output) in tests {
            let output = parse_weekdays(input);
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_nweekdays() {
        let tests = ["", "    ", "fjoasfjapsjop", "MONDAY", "MONDAY, TUESDAY"];

        for input in tests {
            let res = parse_weekdays(input);
            assert!(res.is_err());
        }
    }

    #[test]
    fn parses_valid_weekdays() {
        let tests = [
            ("MO", Weekday::Monday),
            ("TU", Weekday::Tuesday),
            ("WE", Weekday::Wednesday),
            ("TH", Weekday::Thursday),
            ("FR", Weekday::Friday),
            ("SA", Weekday::Saturday),
            ("SU", Weekday::Sunday),
        ];

        for (input, expected_output) in tests {
            let output = str_to_weekday(input);
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_weekdays() {
        let tests = ["", "    ", "fjoasfjapsjop", "MONDAY", "MONDAY, TUESDAY"];

        for input in tests {
            let res = str_to_weekday(input);
            assert!(res.is_err());
        }
    }

    #[test]
    fn parses_valid_datestime_str() {
        let tests = [
            (
                "19970902T090000Z",
                None,
                Tz::UTC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
            (
                "19970902T090000",
                Some(Tz::UTC),
                Tz::UTC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
            (
                "19970902T090000",
                Some(US_PACIFIC),
                US_PACIFIC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
            (
                "19970902T090000Z",
                Some(US_PACIFIC),
                // Timezone is overwritten by the zulu specified in the datetime string
                Tz::UTC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
        ];

        for (datetime_str, timezone, expected_output) in tests {
            let output =
                datestring_to_date(datetime_str, timezone.map(Tz::zone).as_ref(), "DTSTART");
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_datetime_str() {
        let tests = [
            ("", None),
            ("TZID=America/New_York:19970902T090000", None),
            ("19970902T09", None),
            ("19970902T09", Some(US_PACIFIC)),
        ];

        for (datetime_str, timezone) in tests {
            let res = datestring_to_date(datetime_str, timezone.map(Tz::zone).as_ref(), "DTSTART");
            assert!(res.is_err());
        }
    }
}
