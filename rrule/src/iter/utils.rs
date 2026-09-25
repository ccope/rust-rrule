use std::ops;

use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::Zoned;

/// The date `ordinal` days after 1970-01-01, or `None` outside jiff's date range.
///
/// Integer arithmetic (Hinnant's `civil_from_days`) rather than a timestamp, which
/// covers fewer dates than `Date` does at both ends.
pub(crate) fn date_from_ordinal(ordinal: i64) -> Option<Date> {
    let z = ordinal + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Date::new(
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    )
    .ok()
}

/// Days from 1970-01-01 to `date` (Hinnant's `days_from_civil`); exact for every `Date`.
pub(crate) fn days_since_unix_epoch(date: Date) -> i64 {
    let (month, day) = (i64::from(date.month()), i64::from(date.day()));
    let year = i64::from(date.year()) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Returns true if given year is a leap year
pub(crate) fn is_leap_year(year: i32) -> bool {
    // Every 4 years, and every 100 years,
    // but not if dividable by 400.
    year.trailing_zeros() >= 2 && (year % 25 != 0 || year.trailing_zeros() >= 4)
}

/// Returns number of days in year,
/// So 365 or 366 depending on the year
pub(crate) fn get_year_len(year: i32) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

pub(crate) trait DifferentSigns {
    fn different_sign(a: Self, b: Self) -> bool;
}

macro_rules! impl_different_signs {
    ($($ty:ty),*) => {
        $(
        impl DifferentSigns for $ty {
            fn different_sign(a: Self, b: Self) -> bool {
                a > 0 && b < 0 || a < 0 && b > 0
            }
        })*
    };
    (@unsigned, $($ty:ty),*) => {
        $(
        impl DifferentSigns for $ty {
            fn different_sign(_: Self, _: Self) -> bool {
                false
            }
        })*
    };
}
impl_different_signs!(isize, i32, i16);
impl_different_signs!(@unsigned, usize, u32, u16, u8);

pub(crate) fn pymod<T>(a: T, b: T) -> T
where
    T: DifferentSigns + Copy + ops::Rem<Output = T> + ops::Add<Output = T>,
{
    let r = a % b;
    // If r and b differ in sign, add b to wrap the result to the correct sign.
    if T::different_sign(r, b) {
        r + b
    } else {
        r
    }
}

/// Places `time` on `date` in `tz`, resolving a time the clock skips (a DST gap)
/// by the offset before the gap and a time it repeats (a fold) by the earlier
/// occurrence, as RFC 5545 §3.3.5 requires.
pub(crate) fn add_time_to_date(tz: &TimeZone, date: Date, time: Time) -> Option<Zoned> {
    tz.to_zoned(date.to_datetime(time)).ok()
}
#[cfg(test)]
mod test {

    use crate::tests::compat::Tz;

    use super::*;

    #[test]
    fn naive_date_from_ordinal() {
        let tests = [
            (-1, jiff::civil::date(1969, 12, 31)),
            (0, jiff::civil::date(1970, 1, 1)),
            (1, jiff::civil::date(1970, 1, 2)),
            (10, jiff::civil::date(1970, 1, 11)),
            (365, jiff::civil::date(1971, 1, 1)),
            (19877, jiff::civil::date(2024, 6, 3)),
        ];

        for (days, expected) in tests {
            assert_eq!(date_from_ordinal(days), Some(expected), "days: {}", days);
        }
    }

    #[test]
    fn ordinals_round_trip_across_the_whole_date_range() {
        for date in [
            jiff::civil::Date::MIN,
            jiff::civil::date(-1, 2, 28),
            jiff::civil::date(0, 2, 29),
            jiff::civil::date(1900, 3, 1),
            jiff::civil::date(2000, 2, 29),
            jiff::civil::Date::MAX,
        ] {
            let ordinal = days_since_unix_epoch(date);
            assert_eq!(date_from_ordinal(ordinal), Some(date), "{date}");
        }
        let past_max = days_since_unix_epoch(jiff::civil::Date::MAX) + 1;
        let before_min = days_since_unix_epoch(jiff::civil::Date::MIN) - 1;
        assert_eq!(date_from_ordinal(past_max), None);
        assert_eq!(date_from_ordinal(before_min), None);
    }

    #[test]
    fn python_mod() {
        assert_eq!(pymod(2, -3), -1);
        assert_eq!(pymod(-2, 3), 1);
        assert_eq!(pymod(-2, -3), -2);
        assert_eq!(pymod(-3, -3), 0);
        assert_eq!(pymod(0, 3), 0);
        assert_eq!(pymod(1, 3), 1);
        assert_eq!(pymod(2, 3), 2);
        assert_eq!(pymod(3, 3), 0);
        assert_eq!(pymod(4, 3), 1);
        assert_eq!(pymod(6, 3), 0);
        assert_eq!(pymod(-6, 3), 0);
        assert_eq!(pymod(-6, -3), 0);
        assert_eq!(pymod(6, -3), 0);
    }

    #[test]
    fn leap_year() {
        let tests = [
            (2015, false),
            (2016, true),
            (2017, false),
            (2018, false),
            (2019, false),
            (2020, true),
            (2021, false),
        ];

        for (year, expected_output) in tests {
            let res = is_leap_year(year);
            assert_eq!(res, expected_output);
        }
    }

    #[test]
    fn year_length() {
        let tests = [(2015, 365), (2016, 366)];

        for (year, expected_output) in tests {
            let res = get_year_len(year);
            assert_eq!(res, expected_output);
        }
    }

    #[test]
    fn adds_time_to_date() {
        let tests = [
            (
                Tz::UTC,
                jiff::civil::date(2017, 1, 1),
                jiff::civil::time(1, 15, 30, 0),
                Some(Tz::UTC.with_ymd_and_hms(2017, 1, 1, 1, 15, 30).unwrap()),
            ),
            (
                Tz::America__Vancouver,
                jiff::civil::date(2021, 3, 14),
                jiff::civil::time(2, 22, 10, 0),
                Some(
                    Tz::America__Vancouver
                        .with_ymd_and_hms(2021, 3, 14, 0, 0, 0)
                        .unwrap()
                        .checked_add(jiff::SignedDuration::from_secs(2 * 3600 + 22 * 60 + 10))
                        .unwrap(),
                ),
            ),
            (
                Tz::America__New_York,
                jiff::civil::date(1997, 10, 26),
                jiff::civil::time(9, 0, 0, 0),
                Some(
                    Tz::America__New_York
                        .with_ymd_and_hms(1997, 10, 26, 9, 0, 0)
                        .unwrap(),
                ),
            ),
        ];

        for (tz, date, time, expected_output) in tests {
            let res = add_time_to_date(&tz.zone(), date, time);
            assert_eq!(res, expected_output);
        }
    }
}
