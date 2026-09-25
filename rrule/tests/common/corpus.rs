//! Parses the corpus format described in tests/data/README.md and checks each case
//! through `adapter`. Knows nothing about the crate's types.
use super::adapter::{self, Expansion, Format, Query};

#[derive(Default)]
struct Case {
    line: usize,
    reference: String,
    tz: String,
    dtstart: String,
    dtstart_always: bool,
    lines: Vec<String>,
    advance_to: Option<String>,
    after: Option<String>,
    window: Option<(String, String)>,
    limit: Option<usize>,
    format: Option<Format>,
    instances: Option<Vec<String>>,
    more: Option<String>,
    count: Option<usize>,
    nth: Option<(usize, String)>,
    same_as: Option<String>,
}

/// Instances fetched when nothing else bounds a case.
const SAFETY_CAP: usize = 10_000;

fn parse(src: &str) -> Result<Vec<Case>, String> {
    let mut out: Vec<Case> = vec![];
    for (i, raw) in src.lines().enumerate() {
        let l = raw.trim_end_matches('\r');
        if l.trim().is_empty() || l.starts_with('#') {
            continue;
        }
        let (key, value) = l
            .split_once(':')
            .ok_or_else(|| format!("line {}: no ':'", i + 1))?;
        if key == "CASE" {
            out.push(Case {
                line: i + 1,
                reference: value.into(),
                tz: "UTC".into(),
                ..Case::default()
            });
            continue;
        }
        let c = out
            .last_mut()
            .ok_or_else(|| format!("line {}: {key} before any CASE", i + 1))?;
        let num = |v: &str| {
            v.parse::<usize>()
                .map_err(|e| format!("line {}: {e}", i + 1))
        };
        match key {
            "TZ" => c.tz = value.into(),
            "DTSTART" => c.dtstart = value.into(),
            "DTSTART-INSTANCE" => c.dtstart_always = value == "always",
            "ADVANCE-TO" => c.advance_to = Some(value.into()),
            "AFTER" => c.after = Some(value.into()),
            "WINDOW" => {
                let (a, b) = value
                    .split_once('/')
                    .ok_or_else(|| format!("line {}: WINDOW needs a/b", i + 1))?;
                c.window = Some((a.into(), b.into()));
            }
            "LIMIT" => c.limit = Some(num(value)?),
            "INSTANCES-AS" => {
                c.format = Some(match value {
                    "utc" => Format::Utc,
                    "utc-z" => Format::UtcZ,
                    "local" => Format::Local,
                    "local-offset" => Format::LocalOffset,
                    o => return Err(format!("line {}: INSTANCES-AS {o}", i + 1)),
                })
            }
            "INSTANCES" => {
                c.instances = Some(
                    value
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect(),
                )
            }
            "MORE" => c.more = Some(value.into()),
            "COUNT" => c.count = Some(num(value)?),
            "NTH" => {
                let (n, v) = value
                    .split_once('=')
                    .ok_or_else(|| format!("line {}: NTH needs i=value", i + 1))?;
                c.nth = Some((num(n)?, v.into()));
            }
            "SAME-AS" => c.same_as = Some(value.into()),
            _ => {
                let name = key.split(';').next().unwrap_or_default();
                if ["RRULE", "EXRULE", "RDATE", "EXDATE"].contains(&name) {
                    c.lines.push(l.into());
                } else {
                    return Err(format!("line {}: unknown key {key}", i + 1));
                }
            }
        }
    }
    Ok(out)
}

enum Outcome {
    Pass,
    Fail(String),
    Error(String),
}

fn query<'a>(c: &'a Case, lines: &'a [String], max: usize) -> Query<'a> {
    let lower = match (&c.advance_to, &c.after, &c.window) {
        (Some(a), _, _) => Some((a.as_str(), true)),
        (_, Some(a), _) => Some((a.as_str(), false)),
        (_, _, Some((a, _))) => Some((a.as_str(), true)),
        _ => None,
    };
    Query {
        tz: &c.tz,
        dtstart: &c.dtstart,
        dtstart_always: c.dtstart_always,
        lines,
        lower,
        upper: c.window.as_ref().map(|(_, b)| b.as_str()),
        max,
        format: c.format.unwrap_or(Format::Utc),
    }
}

fn run_guarded(q: &Query) -> Result<Expansion, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| adapter::expand(q))).unwrap_or_else(
        |p| {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            Err(format!("panic: {msg}"))
        },
    )
}

fn first_diff(exp: &[String], got: &[String]) -> String {
    let i = exp
        .iter()
        .zip(got)
        .position(|(a, b)| a != b)
        .unwrap_or(exp.len().min(got.len()));
    let mut s = format!(
        "expected {} got {} | first diff at {i}: exp {:?} got {:?}",
        exp.len(),
        got.len(),
        exp.get(i),
        got.get(i)
    );
    if exp.len().max(got.len()) <= 12 {
        s += &format!("\n     exp {}\n     got {}", exp.join(","), got.join(","));
    }
    s
}

fn check(c: &Case) -> (Outcome, bool) {
    // Fetch one past LIMIT so MORE can be judged; otherwise enough to see a COUNT overshoot.
    let max = match (c.limit, c.count, &c.nth, &c.instances) {
        (Some(l), ..) => l + 1,
        (None, Some(n), ..) => n + 1,
        (None, None, _, Some(v)) => v.len() + 1,
        (None, None, Some((n, _)), None) => n + 1,
        _ => SAFETY_CAP,
    };
    let got = match run_guarded(&query(c, &c.lines, max)) {
        Ok(e) => e,
        Err(e) => return (Outcome::Error(e), false),
    };
    let limited = got.hit_iteration_limit;
    let mut got = got.instances;
    let more = c.limit.is_some_and(|l| got.len() > l);
    if let Some(l) = c.limit {
        got.truncate(l);
    }

    let mut problems = vec![];
    if let Some(same) = &c.same_as {
        let lines: Vec<String> = c
            .lines
            .iter()
            .map(|l| {
                if l.starts_with("RRULE:") {
                    format!("RRULE:{same}")
                } else {
                    l.clone()
                }
            })
            .collect();
        match run_guarded(&query(c, &lines, max)) {
            Ok(e) if e.instances != got => problems.push(format!(
                "SAME-AS {same}: {}",
                first_diff(&e.instances, &got)
            )),
            Ok(_) => {}
            Err(e) => return (Outcome::Error(format!("SAME-AS rule: {e}")), limited),
        }
    }
    if let Some(exp) = &c.instances {
        if *exp != got {
            problems.push(first_diff(exp, &got));
        }
    }
    match c.more.as_deref() {
        Some("yes") if !more => {
            problems.push("expected more instances past LIMIT, got none".into())
        }
        Some("no") | None if c.limit.is_some() && more => {
            problems.push("expected no instances past LIMIT, got more".into())
        }
        _ => {}
    }
    if let Some(n) = c.count {
        if got.len() != n {
            problems.push(format!("expected COUNT {n}, got {}", got.len()));
        }
    }
    if let Some((i, v)) = &c.nth {
        if got.get(*i) != Some(v) {
            problems.push(format!("expected [{i}] = {v}, got {:?}", got.get(*i)));
        }
    }
    let outcome = if problems.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(problems.join("; "))
    };
    (outcome, limited)
}

/// Runs every case in the corpus at `path` and prints one line per non-passing case plus
/// a summary. Reports rather than asserts: a failing case does not fail `cargo test`.
pub fn run(name: &str, path: &str) {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let cases = parse(&src).unwrap_or_else(|e| panic!("{path}: {e}"));
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let (mut pass, mut fail, mut err) = (0, 0, 0);
    for c in &cases {
        let (outcome, limited) = check(c);
        let note = if limited {
            " (crate hit its iteration limit)"
        } else {
            ""
        };
        match outcome {
            Outcome::Pass => pass += 1,
            Outcome::Fail(msg) => {
                fail += 1;
                println!(
                    "FAIL line {} [{}] {}{note}\n     DTSTART {} TZ {} | {msg}",
                    c.line,
                    c.reference,
                    c.lines.join(" / "),
                    c.dtstart,
                    c.tz
                );
            }
            Outcome::Error(msg) => {
                err += 1;
                println!(
                    "ERR  line {} [{}] {}: {msg}",
                    c.line,
                    c.reference,
                    c.lines.join(" / ")
                );
            }
        }
    }
    std::panic::set_hook(prev_hook);
    println!(
        "\n{name} corpus: {} cases, {pass} pass, {fail} fail, {err} errors",
        cases.len()
    );
}
