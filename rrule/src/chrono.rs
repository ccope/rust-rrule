//! Conversions between [`Zoned`] and chrono date-times, for callers still on chrono.
//!
//! The recurrence engine works in jiff; these functions only translate values
//! at the boundary. Zones are matched by IANA name, so a conversion fails when
//! one side's time zone database does not know the other side's zone.

use chrono::{DateTime, FixedOffset, Utc};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

/// Why a value could not be converted.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConversionError {
    /// The instant is outside the range the target library supports.
    #[error("`{0}` is out of range for the target date-time type")]
    OutOfRange(String),
    /// The zone name is not in the target library's time zone database.
    #[error("time zone `{0}` is not in the target time zone database")]
    UnknownZone(String),
    /// The zone has no IANA name (a fixed offset, or floating), so there is no zone to name.
    #[error("the value's time zone has no IANA name")]
    UnnamedZone,
}

fn timestamp_of<Tz: chrono::TimeZone>(dt: &DateTime<Tz>) -> Result<Timestamp, ConversionError> {
    let nanos = i32::try_from(dt.timestamp_subsec_nanos())
        .map_err(|_| ConversionError::OutOfRange(dt.to_rfc3339()))?;
    Timestamp::new(dt.timestamp(), nanos).map_err(|_| ConversionError::OutOfRange(dt.to_rfc3339()))
}

/// Converts a chrono-tz date-time, keeping its IANA zone.
pub fn from_chrono_tz(dt: &DateTime<chrono_tz::Tz>) -> Result<Zoned, ConversionError> {
    let name = dt.timezone().name();
    let tz = TimeZone::get(name).map_err(|_| ConversionError::UnknownZone(name.into()))?;
    Ok(timestamp_of(dt)?.to_zoned(tz))
}

/// Converts a UTC date-time.
pub fn from_utc(dt: &DateTime<Utc>) -> Result<Zoned, ConversionError> {
    Ok(timestamp_of(dt)?.to_zoned(TimeZone::UTC))
}

/// Converts `zdt` to chrono-tz, keeping its IANA zone.
pub fn to_chrono_tz(zdt: &Zoned) -> Result<DateTime<chrono_tz::Tz>, ConversionError> {
    let name = zdt
        .time_zone()
        .iana_name()
        .ok_or(ConversionError::UnnamedZone)?;
    let tz: chrono_tz::Tz = name
        .parse()
        .map_err(|_| ConversionError::UnknownZone(name.into()))?;
    Ok(to_fixed_offset(zdt)?.with_timezone(&tz))
}

/// Converts `zdt` to its instant with its UTC offset. Works for any zone, floating included.
pub fn to_fixed_offset(zdt: &Zoned) -> Result<DateTime<FixedOffset>, ConversionError> {
    let offset = FixedOffset::east_opt(zdt.offset().seconds())
        .ok_or_else(|| ConversionError::OutOfRange(zdt.to_string()))?;
    let ts = zdt.timestamp();
    let nanos = u32::try_from(ts.subsec_nanosecond())
        .map_err(|_| ConversionError::OutOfRange(zdt.to_string()))?;
    DateTime::<Utc>::from_timestamp(ts.as_second(), nanos)
        .map(|utc| utc.with_timezone(&offset))
        .ok_or_else(|| ConversionError::OutOfRange(zdt.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone as _;

    #[test]
    fn round_trips_a_named_zone() {
        let berlin = chrono_tz::Europe::Berlin
            .with_ymd_and_hms(2021, 3, 28, 3, 30, 0)
            .unwrap();
        let zoned = from_chrono_tz(&berlin).unwrap();
        assert_eq!(zoned.time_zone().iana_name(), Some("Europe/Berlin"));
        assert_eq!(
            zoned.to_string(),
            "2021-03-28T03:30:00+02:00[Europe/Berlin]"
        );
        assert_eq!(to_chrono_tz(&zoned).unwrap(), berlin);
    }

    #[test]
    fn floating_converts_only_to_a_fixed_offset() {
        let set: crate::RRuleSet = "DTSTART:20210313T023000\nRRULE:FREQ=DAILY;COUNT=1"
            .parse()
            .unwrap();
        let floating = set.get_dt_start();
        assert_eq!(to_chrono_tz(floating), Err(ConversionError::UnnamedZone));
        assert_eq!(
            to_fixed_offset(floating).unwrap().to_rfc3339(),
            "2021-03-13T02:30:00+00:00"
        );
    }

    #[test]
    fn converts_utc() {
        let utc = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap();
        assert_eq!(
            from_utc(&utc).unwrap().to_string(),
            "2024-01-02T03:04:05+00:00[UTC]"
        );
    }
}
