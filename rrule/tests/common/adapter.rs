//! The only code in the corpus runners that touches the `rrule` crate's API.
//! Porting the crate to another date-time library means rewriting this file;
//! `corpus.rs` stays as it is.
use jiff::civil::DateTime;
use jiff::tz::{AmbiguousOffset, TimeZone};
use rrule::{RRuleSet, Zoned};

/// How an expected instance is printed (the corpus `INSTANCES-AS` key).
#[derive(Clone, Copy, Debug)]
pub enum Format {
    /// UTC wall clock, `YYYYMMDDTHHMMSS` with no trailing `Z`.
    Utc,
    /// UTC wall clock with a trailing `Z`.
    UtcZ,
    /// Wall clock in the case's zone, `YYYYMMDDTHHMMSS`.
    Local,
    /// Wall clock in the case's zone plus its UTC offset, `YYYYMMDDTHHMMSS-0800`.
    LocalOffset,
}

/// One expansion request, in corpus terms (plain strings, no crate types).
pub struct Query<'a> {
    pub tz: &'a str,
    pub dtstart: &'a str,
    /// DTSTART is an instance even when the rules do not generate it.
    pub dtstart_always: bool,
    /// RRULE / EXRULE / RDATE / EXDATE content lines, verbatim from the corpus.
    pub lines: &'a [String],
    /// Lower bound and whether it is inclusive.
    pub lower: Option<(&'a str, bool)>,
    /// Inclusive upper bound.
    pub upper: Option<&'a str>,
    /// Stop after this many instances.
    pub max: usize,
    pub format: Format,
}

pub struct Expansion {
    pub instances: Vec<String>,
    /// The crate stopped early on its own loop guard rather than running out of instances.
    pub hit_iteration_limit: bool,
}

/// Beyond `u16::MAX` the crate's `all()` cannot be used; iterate by hand, but give up
/// after this many raw instances so a runaway case cannot stall the run.
const RAW_BUDGET: usize = 3_000_000;

pub fn expand(q: &Query) -> Result<Expansion, String> {
    let date_only = q.dtstart.len() == 8;
    // DATE values are zone-less: evaluate them as midnight UTC whatever TZ says.
    let zone_name = if date_only { "UTC" } else { q.tz };
    let zone = TimeZone::get(zone_name).map_err(|e| format!("unknown TZ {}: {e}", q.tz))?;

    let params = if date_only {
        ";VALUE=DATE;TZID=UTC".to_string()
    } else if q.dtstart.ends_with('Z') {
        String::new()
    } else {
        format!(";TZID={zone_name}")
    };
    let mut text = format!("DTSTART{params}:{}", q.dtstart);
    for line in q.lines {
        text.push('\n');
        text.push_str(&rewrite_line(line, &zone, zone_name)?);
    }
    if q.dtstart_always {
        // As text rather than RRuleSet::rdate so a set with no rules of its own parses.
        text.push_str(&format!("\nRDATE{params}:{}", q.dtstart));
    }
    let mut set: RRuleSet = text.parse().map_err(|e| format!("{e}"))?;
    let dtstart = set.get_dt_start().clone();

    let lower = q
        .lower
        .map(|(v, inc)| at(v, &zone).map(|d| (d, inc)))
        .transpose()?;
    let upper = q.upper.map(|v| at(v, &zone)).transpose()?;
    // An exclusive bound is asked for inclusively, and the injected DTSTART may be
    // generated twice; fetch enough extra to cover both.
    let want =
        q.max + usize::from(matches!(lower, Some((_, false)))) + usize::from(q.dtstart_always);

    let (mut dates, hit_iteration_limit) = if want <= usize::from(u16::MAX) {
        if let Some((l, _)) = &lower {
            set = set.after(l.clone());
        }
        if let Some(u) = &upper {
            set = set.before(u.clone());
        }
        let r = set.all(want as u16);
        let early = r.limited && r.dates.len() < want;
        (r.dates, early)
    } else {
        let mut out = vec![];
        for (n, d) in set.limit().into_iter().enumerate() {
            if n >= RAW_BUDGET {
                return Err(format!("runner gave up after {RAW_BUDGET} raw instances"));
            }
            if upper.as_ref().is_some_and(|u| d > *u) || out.len() >= want {
                break;
            }
            if lower.as_ref().map_or(true, |(l, _)| d >= *l) {
                out.push(d);
            }
        }
        (out, false)
    };

    if let Some((l, false)) = &lower {
        dates.retain(|d| d != l);
    }
    if q.dtstart_always {
        // The injected RDATE must not duplicate a DTSTART the rules generate too.
        if dates.iter().filter(|d| **d == dtstart).count() > 1 {
            let i = dates.iter().position(|d| *d == dtstart).unwrap();
            dates.remove(i);
        }
    }
    dates.truncate(q.max);

    Ok(Expansion {
        instances: dates.iter().map(|d| fmt(d, date_only, q.format)).collect(),
        hit_iteration_limit,
    })
}

fn fmt(d: &Zoned, date_only: bool, f: Format) -> String {
    if date_only {
        return d.strftime("%Y%m%d").to_string();
    }
    let utc = d.with_time_zone(TimeZone::UTC);
    match f {
        Format::Utc => utc.strftime("%Y%m%dT%H%M%S").to_string(),
        Format::UtcZ => utc.strftime("%Y%m%dT%H%M%SZ").to_string(),
        Format::Local => d.strftime("%Y%m%dT%H%M%S").to_string(),
        Format::LocalOffset => d.strftime("%Y%m%dT%H%M%S%z").to_string(),
    }
}

/// A corpus value (`YYYYMMDD`, floating `YYYYMMDDTHHMMSS`, or UTC `...Z`) as an instant.
/// A repeated local time takes its earlier instant; a skipped one is an error.
fn at(v: &str, zone: &TimeZone) -> Result<Zoned, String> {
    let bad = |e: jiff::Error| format!("bad value {v}: {e}");
    if let Some(u) = v.strip_suffix('Z') {
        let n = DateTime::strptime("%Y%m%dT%H%M%S", u).map_err(bad)?;
        return n.to_zoned(TimeZone::UTC).map_err(bad);
    }
    let n = if v.len() == 8 {
        jiff::civil::Date::strptime("%Y%m%d", v)
            .map_err(bad)?
            .to_datetime(jiff::civil::Time::midnight())
    } else {
        DateTime::strptime("%Y%m%dT%H%M%S", v).map_err(bad)?
    };
    let ambiguous = zone.to_ambiguous_zoned(n);
    if let AmbiguousOffset::Gap { .. } = ambiguous.offset() {
        return Err(format!(
            "{v} does not exist in {}",
            zone.iana_name().unwrap_or("its zone")
        ));
    }
    ambiguous.earlier().map_err(bad)
}

/// Make a corpus content line acceptable to the crate: a floating or DATE `UNTIL`
/// becomes the UTC instant it denotes in the case's zone (the crate requires UTC
/// when DTSTART has a TZID), and RDATE/EXDATE without a TZID get the case's zone
/// (the crate would otherwise use the machine's local zone).
fn rewrite_line(line: &str, zone: &TimeZone, zone_name: &str) -> Result<String, String> {
    let (head, value) = line
        .split_once(':')
        .ok_or_else(|| format!("bad line {line}"))?;
    let name = head
        .split(';')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    match name.as_str() {
        "RRULE" | "EXRULE" => {
            #[cfg(not(feature = "exrule"))]
            if name == "EXRULE" {
                return Err(
                    "EXRULE needs the crate's `exrule` feature (run with --features exrule)".into(),
                );
            }
            let parts: Result<Vec<String>, String> = value
                .split(';')
                .map(|p| match p.strip_prefix("UNTIL=") {
                    Some(u) if !u.ends_with('Z') => Ok(format!(
                        "UNTIL={}",
                        at(u, zone)?
                            .with_time_zone(TimeZone::UTC)
                            .strftime("%Y%m%dT%H%M%SZ")
                    )),
                    _ => Ok(p.to_string()),
                })
                .collect();
            Ok(format!("{head}:{}", parts?.join(";")))
        }
        "RDATE" | "EXDATE" => {
            let has_tzid = head.to_ascii_uppercase().contains(";TZID=");
            if has_tzid || value.split(',').all(|v| v.ends_with('Z')) {
                Ok(line.to_string())
            } else {
                Ok(format!("{head};TZID={zone_name}:{value}"))
            }
        }
        _ => Err(format!("unsupported content line {line}")),
    }
}
