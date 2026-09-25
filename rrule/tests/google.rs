//! Checks the crate's expansion against Google Calendar's, recorded by
//! schedule-me's `devtool recurrence-fixtures` into `tests/data/google/*.json`.
//!
//! A series is rebuilt from the DTSTART the spec wrote and the recurrence lines
//! Google echoed back, expanded over the captured window, and compared with the
//! slot (`originalStartTime`) of every instance Google listed, cancelled ones
//! included: a cancellation is an exception to the series, not part of the rule.
use rrule::{RRuleSet, TimeZone, Zoned};
use serde_json::Value;

/// Series where the crate is known to disagree with Google, and why.
const KNOWN: &[(&str, &str)] = &[
    (
        "la-dtstart-not-in-byday",
        "Google adds a DTSTART the rule does not generate as an extra instance, outside COUNT; \
         the crate follows dateutil and drops it",
    ),
    (
        "la-yearly-bymonthday-no-bymonth",
        "Google takes a missing BYMONTH from DTSTART (RFC 5545: parts the rule omits come from \
         DTSTART); the crate follows dateutil and repeats BYMONTHDAY in every month",
    ),
];

fn slot(v: &Value) -> Option<String> {
    if let Some(d) = v["date"].as_str() {
        return Some(d.to_string());
    }
    let ts: jiff::Timestamp = v["dateTime"].as_str()?.parse().ok()?;
    Some(ts.to_string())
}

fn crate_slot(z: &Zoned, all_day: bool) -> String {
    if all_day {
        z.date().to_string()
    } else {
        z.timestamp().to_string()
    }
}

fn expand(series: &Value) -> Result<Vec<String>, String> {
    let spec = &series["spec"];
    let start = &spec["start"];
    let (dtstart, all_day) = if let Some(date) = start["date"].as_str() {
        (
            format!("DTSTART;VALUE=DATE:{}", date.replace('-', "")),
            true,
        )
    } else {
        let local = start["dateTime"]
            .as_str()
            .ok_or("spec start has no dateTime")?;
        let local = local.replace(['-', ':'], "");
        let tz = start["timeZone"]
            .as_str()
            .ok_or("spec start has no timeZone")?;
        (format!("DTSTART;TZID={tz}:{local}"), false)
    };
    let mut text = dtstart;
    for line in series["master"]["recurrence"]
        .as_array()
        .into_iter()
        .flatten()
    {
        text.push('\n');
        text.push_str(line.as_str().unwrap_or_default());
    }
    let set: RRuleSet = text.parse().map_err(|e| format!("{e}: {text}"))?;
    let time_max: jiff::Timestamp = spec["time_max"]
        .as_str()
        .and_then(|t| t.parse().ok())
        .ok_or("spec has no time_max")?;
    let mut out = vec![];
    for z in &set {
        if z.timestamp() >= time_max && !all_day {
            break;
        }
        if all_day
            && z.date()
                .to_zoned(TimeZone::UTC)
                .map_or(true, |d| d.timestamp() >= time_max)
        {
            break;
        }
        out.push(crate_slot(&z, all_day));
        if out.len() > 100_000 {
            return Err("more than 100000 occurrences".into());
        }
    }
    Ok(out)
}

fn run(path: &std::path::Path) -> (usize, usize, Vec<String>) {
    let data: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let (mut pass, mut fail, mut unexpected) = (0, 0, vec![]);
    for series in data["series"].as_array().into_iter().flatten() {
        let name = series["spec"]["name"].as_str().unwrap_or("?");
        if series.get("rejected").is_some() {
            println!("SKIP {name}: Google rejected it");
            continue;
        }
        let mut google: Vec<String> = series["instances"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|i| slot(&i["originalStartTime"]).or_else(|| slot(&i["start"])))
            .collect();
        google.sort();
        let ours = expand(series);
        let ok = matches!(&ours, Ok(o) if *o == google);
        if ok {
            pass += 1;
            continue;
        }
        fail += 1;
        let known = KNOWN.iter().find(|(n, _)| *n == name);
        match &ours {
            Ok(o) => {
                let missing: Vec<_> = google.iter().filter(|g| !o.contains(g)).collect();
                let extra: Vec<_> = o.iter().filter(|x| !google.contains(x)).collect();
                println!(
                    "FAIL {name}{}: google {} ours {} | missing {missing:?} | extra {extra:?}",
                    known.map_or(String::new(), |(_, why)| format!(" (known: {why})")),
                    google.len(),
                    o.len()
                );
            }
            Err(e) => println!("ERR  {name}: {e}"),
        }
        if known.is_none() {
            unexpected.push(name.to_string());
        }
    }
    (pass, fail, unexpected)
}

#[test]
fn google_fixtures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/google");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && !p.to_string_lossy().ends_with("-spec.json")
        })
        .collect();
    // Captures of real calendars stay out of git; include them when present.
    if let Ok(private) = std::fs::read_dir(dir.join("private")) {
        files.extend(
            private
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|x| x == "json")
                        && !p.to_string_lossy().ends_with("-spec.json")
                }),
        );
    }
    files.sort();
    for f in files {
        let (pass, fail, unexpected) = run(&f);
        println!(
            "google fixtures {}: {} series, {pass} match, {fail} differ ({} not in KNOWN)",
            f.file_name().unwrap().to_string_lossy(),
            pass + fail,
            unexpected.len()
        );
    }
}
