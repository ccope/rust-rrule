//! Runs libical's recurrence corpus (tests/data/libical_icalrecur_test.txt, MPL-2.0) against this crate.
use rrule::{RRuleSet, Zoned};

struct Case {
    line: usize,
    comment: String,
    rrule: String,
    dtstart: String,
    start_at: Option<String>,
    instances: Vec<String>,
}

fn parse(src: &str) -> Vec<Case> {
    let mut out = vec![];
    let mut cur: Option<Case> = None;
    let mut comment = String::new();
    for (i, l) in src.lines().enumerate() {
        if let Some(c) = l.strip_prefix('#') {
            comment = c.trim().to_string();
            continue;
        }
        if let Some(r) = l.strip_prefix("RRULE:") {
            if let Some(c) = cur.take() {
                out.push(c);
            }
            cur = Some(Case {
                line: i + 1,
                comment: comment.clone(),
                rrule: r.into(),
                dtstart: String::new(),
                start_at: None,
                instances: vec![],
            });
        } else if let Some(v) = l.strip_prefix("DTSTART:") {
            cur.as_mut().unwrap().dtstart = v.into();
        } else if let Some(v) = l.strip_prefix("START-AT:") {
            cur.as_mut().unwrap().start_at = Some(v.into());
        } else if let Some(v) = l.strip_prefix("INSTANCES:") {
            cur.as_mut().unwrap().instances = v
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
        }
    }
    if let Some(c) = cur {
        out.push(c);
    }
    out
}

fn fmt(dt: &Zoned, date_only: bool, utc: bool) -> String {
    if date_only {
        dt.strftime("%Y%m%d").to_string()
    } else if utc {
        dt.strftime("%Y%m%dT%H%M%SZ").to_string()
    } else {
        dt.strftime("%Y%m%dT%H%M%S").to_string()
    }
}

#[test]
fn libical_corpus() {
    let path = std::env::var("LIBICAL_RECUR").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/libical_icalrecur_test.txt"
        )
        .into()
    });
    let cases = parse(&std::fs::read_to_string(path).unwrap());
    let (mut pass, mut fail, mut err) = (0, 0, 0);
    for c in &cases {
        let date_only = !c.dtstart.contains('T');
        let utc = c.dtstart.ends_with('Z');
        let dt = if date_only {
            format!("DTSTART;VALUE=DATE:{}", c.dtstart)
        } else if utc {
            format!("DTSTART:{}", c.dtstart)
        } else {
            format!("DTSTART;TZID=UTC:{}", c.dtstart)
        };
        // Floating DTSTART is run as UTC, so a floating UNTIL must be UTC too.
        let rule = if !utc && !date_only {
            fix_until(&c.rrule)
        } else {
            c.rrule.clone()
        };
        let set: Result<RRuleSet, _> = format!("{dt}\nRRULE:{rule}").parse();
        let set = match set {
            Ok(s) => s,
            Err(e) => {
                err += 1;
                println!("ERR  line {} [{}] {}: {e}", c.line, c.comment, c.rrule);
                continue;
            }
        };
        let set = match &c.start_at {
            Some(s) => {
                let s: RRuleSet = format!(
                    "DTSTART;TZID=UTC:{}\nRRULE:FREQ=DAILY;COUNT=1",
                    s.trim_end_matches('Z')
                )
                .parse()
                .unwrap();
                set.after(s.get_dt_start().clone())
            }
            None => set,
        };
        let expected: Vec<String> = match &c.start_at {
            Some(s) => c
                .instances
                .iter()
                .filter(|i| i.as_str() >= s.as_str())
                .cloned()
                .collect(),
            None => c.instances.clone(),
        };
        if expected.first().is_some_and(|e| e.starts_with(" ***")) {
            println!(
                "SKIP line {} [{}]: libical expects {}",
                c.line, c.comment, expected[0]
            );
            continue;
        }
        let got: Vec<String> = set
            .all(10_000)
            .dates
            .iter()
            .map(|d| fmt(d, date_only, utc))
            .collect();
        if got == expected {
            pass += 1
        } else {
            fail += 1;
            let first = got.iter().zip(&expected).position(|(a, b)| a != b);
            println!("FAIL line {} [{}] {}\n     DTSTART {} | expected {} got {} | first diff at {:?}: exp {:?} got {:?}",
                c.line, c.comment, c.rrule, c.dtstart, expected.len(), got.len(), first,
                first.map(|i| &expected[i]), first.map(|i| &got[i]));
        }
    }
    println!(
        "\nlibical corpus: {} cases, {pass} pass, {fail} fail, {err} parse errors",
        cases.len()
    );
}

fn fix_until(rule: &str) -> String {
    rule.split(';')
        .map(|p| match p.strip_prefix("UNTIL=") {
            Some(u) if u.contains('T') && !u.ends_with('Z') => format!("UNTIL={u}Z"),
            _ => p.to_string(),
        })
        .collect::<Vec<_>>()
        .join(";")
}
