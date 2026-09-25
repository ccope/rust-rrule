//! chrono-shaped helpers so the upstream tests read as they did before the jiff port.
#![allow(dead_code, non_upper_case_globals)]

use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use jiff::Zoned;

/// A named zone, `Copy` so tests can keep `const BERLIN: Tz = ...`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tz(&'static str);

impl Tz {
    pub const UTC: Self = Self("UTC");
    /// Floating time, as the crate represents a value with no `TZID` and no `Z`.
    pub const LOCAL: Self = Self("");
    pub const America__Los_Angeles: Self = Self("America/Los_Angeles");
    pub const America__New_York: Self = Self("America/New_York");
    pub const America__Vancouver: Self = Self("America/Vancouver");
    pub const Europe__Berlin: Self = Self("Europe/Berlin");

    pub fn zone(self) -> TimeZone {
        match self.0 {
            "" => crate::core::floating(),
            name => TimeZone::get(name).expect("test zones are in the tz database"),
        }
    }

    /// Like chrono's: a local time the zone skips or repeats has no single answer.
    pub fn with_ymd_and_hms(
        self,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Single {
        let dt = DateTime::new(
            i16::try_from(year).unwrap(),
            i8::try_from(month).unwrap(),
            i8::try_from(day).unwrap(),
            i8::try_from(hour).unwrap(),
            i8::try_from(minute).unwrap(),
            i8::try_from(second).unwrap(),
            0,
        );
        Single(
            dt.ok()
                .and_then(|dt| self.zone().to_ambiguous_zoned(dt).unambiguous().ok()),
        )
    }
}

pub struct Single(Option<Zoned>);

impl Single {
    pub fn unwrap(self) -> Zoned {
        self.0
            .expect("local time is valid and unambiguous in its zone")
    }
}

pub trait ToRfc3339 {
    fn to_rfc3339(&self) -> String;
}

impl ToRfc3339 for Zoned {
    fn to_rfc3339(&self) -> String {
        self.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
    }
}
