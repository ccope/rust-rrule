use jiff::tz::TimeZone;
use jiff::Zoned;

/// The zone a floating DATE or DATE-TIME (no `TZID`, no `Z`) is placed in.
///
/// RFC 5545 floating values are not bound to any zone. Jiff's unknown zone
/// behaves like UTC, so floating arithmetic never meets a DST transition, and
/// unlike UTC it can be told apart when the value is written back out.
pub(crate) fn floating() -> TimeZone {
    TimeZone::unknown()
}

pub(crate) fn is_floating(tz: &TimeZone) -> bool {
    tz.is_unknown()
}

pub(crate) fn is_utc(tz: &TimeZone) -> bool {
    tz.iana_name() == Some("UTC") || *tz == TimeZone::UTC
}

pub(crate) fn get_month(dt: &Zoned) -> u8 {
    u8::try_from(dt.month()).expect("month is between 1-12 which is covered by u8")
}

pub(crate) fn get_day(dt: &Zoned) -> i8 {
    dt.day()
}

pub(crate) fn get_hour(dt: &Zoned) -> u8 {
    u8::try_from(dt.hour()).expect("hour is between 0-23 which is covered by u8")
}

pub(crate) fn get_minute(dt: &Zoned) -> u8 {
    u8::try_from(dt.minute()).expect("minute is between 0-59 which is covered by u8")
}

pub(crate) fn get_second(dt: &Zoned) -> u8 {
    u8::try_from(dt.second()).expect("second is between 0-59 which is covered by u8")
}

/// Formats `dt` as the value of an iCalendar DATE-TIME, without the property name.
///
/// UTC is written with a `Z` suffix, a floating value bare, and any other zone
/// with its `TZID` parameter.
pub(crate) fn datetime_to_ical_format(dt: &Zoned) -> String {
    let local = dt.datetime().strftime("%Y%m%dT%H%M%S");
    let tz = dt.time_zone();
    if is_floating(tz) {
        format!(":{local}")
    } else if is_utc(tz) {
        format!(":{local}Z")
    } else {
        match tz.iana_name() {
            Some(name) => format!(";TZID={name}:{local}"),
            // A zone with no name, e.g. a fixed offset, has no TZID to write,
            // so the instant is written in UTC instead.
            None => format!(
                ":{}Z",
                dt.with_time_zone(TimeZone::UTC)
                    .datetime()
                    .strftime("%Y%m%dT%H%M%S")
            ),
        }
    }
}

/// Formats `dt` as a bare DATE-TIME value for a list property (RDATE, EXDATE,
/// UNTIL), where there is no room for a per-value `TZID`: UTC gets a `Z`,
/// floating is written bare, and any other zone is converted to UTC.
pub(crate) fn datetime_to_ical_value(dt: &Zoned) -> String {
    let tz = dt.time_zone();
    if is_floating(tz) {
        dt.datetime().strftime("%Y%m%dT%H%M%S").to_string()
    } else {
        format!(
            "{}Z",
            dt.with_time_zone(TimeZone::UTC)
                .datetime()
                .strftime("%Y%m%dT%H%M%S")
        )
    }
}

/// chrono's weekday numbering, which the dateutil-derived masks are built on.
pub(crate) trait WeekdayExt {
    /// Monday is 0, Sunday is 6.
    fn num_days_from_monday(self) -> u32;
}

impl WeekdayExt for jiff::civil::Weekday {
    fn num_days_from_monday(self) -> u32 {
        u32::try_from(self.to_monday_zero_offset()).expect("0-6 is covered by u32")
    }
}
