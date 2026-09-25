#!/usr/bin/env python3
"""Regenerate ../rfc2445_iter_tests.txt from a google-rfc-2445 checkout.

Usage: rfc2445.py /path/to/google-rfc-2445 > ../rfc2445_iter_tests.txt

Evaluates the arguments of every runRecurrenceIteratorTest(...) call in
RRuleIteratorImplTest.java and CompoundIteratorImplTest.java: Java string
literal concatenation, IcalParseUtil.parseDateValue, new DateValueImpl /
DateTimeValueImpl, goldenDateRange. The handful of tests that do not go through
that helper are transcribed in HAND_CASES below, each with the reason.
"""
import re
import subprocess
import sys
from datetime import date, timedelta
from pathlib import Path

TZ = {"UTC": "UTC", "PST": "America/Los_Angeles", "EST": "America/New_York"}
ITER = "test/com/google/ical/iter/"


def strip_comments(src):
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if c == '"':
            j = i + 1
            while src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            out.append(src[i : j + 1])
            i = j + 1
        elif src.startswith("//", i):
            i = src.index("\n", i)
        elif src.startswith("/*", i):
            i = src.index("*/", i) + 2
            out.append(" ")
        else:
            out.append(c)
            i += 1
    return "".join(out)


def split_top(s, sep):
    parts, depth, cur, i = [], 0, [], 0
    while i < len(s):
        c = s[i]
        if c == '"':
            j = i + 1
            while s[j] != '"':
                j += 2 if s[j] == "\\" else 1
            cur.append(s[i : j + 1])
            i = j + 1
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == sep and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
        i += 1
    parts.append("".join(cur))
    return [p.strip() for p in parts]


def java_str(lit):
    body = lit[1:-1]
    return re.sub(r"\\(.)", lambda m: {"n": "\n", "r": "\r", "t": "\t"}.get(m.group(1), m.group(1)), body)


def golden_date_range(period, interval=1):
    a, b = period.split("/")
    d = date(int(a[:4]), int(a[4:6]), int(a[6:8]))
    end = date(int(b[:4]), int(b[4:6]), int(b[6:8]))
    out = []
    while d <= end:
        out.append(d.strftime("%Y%m%d"))
        d += timedelta(days=interval)
    return ",".join(out)


def ev(expr, env):
    terms = split_top(expr, "+")
    if len(terms) > 1:
        return "".join(ev(t, env) for t in terms)
    e = expr.strip()
    if e.startswith('"'):
        return java_str(e)
    if e in env:
        return env[e]
    if e == "null":
        return None
    if e in TZ:
        return ("TZ", TZ[e])
    if re.fullmatch(r"-?\d+", e):
        return int(e)
    m = re.fullmatch(r"IcalParseUtil\.parseDateValue\((.*)\)", e, re.S)
    if m:
        return ("DV", ev(m.group(1), env))
    m = re.fullmatch(r"new (DateValueImpl|DateTimeValueImpl)\((.*)\)", e, re.S)
    if m:
        f = [int(x) for x in split_top(m.group(2), ",")]
        s = "%04d%02d%02d" % tuple(f[:3])
        if len(f) == 6:
            s += "T%02d%02d%02d" % tuple(f[3:])
        return ("DV", s)
    m = re.fullmatch(r"goldenDateRange\((.*)\)", e, re.S)
    if m:
        args = [ev(a, env) for a in split_top(m.group(1), ",")]
        return golden_date_range(*args)
    raise ValueError(f"cannot evaluate {e!r}")


def methods(src):
    for m in re.finditer(r"public void (test\w+)\(\)\s*(?:throws Exception\s*)?\{", src):
        i, depth = m.end(), 1
        while depth:
            depth += {"{": 1, "}": -1}.get(src[i], 0)
            i += 1
        yield m.group(1), src[m.end() : i - 1]


def calls(body, name="runRecurrenceIteratorTest"):
    for m in re.finditer(re.escape(name) + r"\(", body):
        i, depth = m.end(), 1
        while depth:
            c = body[i]
            if c == '"':
                i += 1
                while body[i] != '"':
                    i += 2 if body[i] == "\\" else 1
            depth += {"(": 1, ")": -1}.get(c, 0)
            i += 1
        yield split_top(body[m.end() : i - 1], ",")


def unfold(text):
    """RFC 2445 4.1 unfolding, then one content line per entry, blank lines dropped."""
    text = re.sub(r"\r?\n[ \t]", "", text)
    return [l for l in re.split(r"\r?\n", text) if l.strip()]


# Upstream expectations that look like artefacts of the implementation rather than
# of RFC 2445; kept as written, flagged here.
NOTES = {
    "RRuleIteratorImplTest.java#testAdvanceTo[12]": [
        "UPSTREAM ARTEFACT: '' because upstream's year generator gives up after a run of",
        "empty years ('advancing way past year generator timeout'); by the rule text",
        "alone, 25000228 is the next instance.",
    ],
}


def emit(out, ref, tz, dtstart, lines, limit, golden, advance_to=None,
         dtstart_instance="if-matches", more=None, notes=()):
    out.append("")
    for n in list(notes) + NOTES.get(ref, []):
        out.append(f"# {n}")
    out.append(f"CASE:{ref}")
    out.append(f"TZ:{tz}")
    out.append(f"DTSTART:{dtstart}")
    out.append(f"DTSTART-INSTANCE:{dtstart_instance}")
    out.extend(lines)
    if advance_to is not None:
        out.append(f"ADVANCE-TO:{advance_to}")
    out.append(f"LIMIT:{limit}")
    vals = golden.split(",") if golden else []
    if more is None:
        more = "no"
        if vals and vals[-1] == "...":
            vals.pop()
            more = "yes"
    out.append("INSTANCES-AS:utc")
    out.append(f"INSTANCES:{','.join(vals)}")
    out.append(f"MORE:{more}")


def utc_advance(dv):
    # RecurrenceIterator.advanceTo takes a UTC value; a DATE stays a DATE.
    return dv + "Z" if "T" in dv else dv


def rrule_iter_cases(src, out):
    n = 0
    for meth, body in methods(src):
        if meth == "testMonthsThatStartOrEndOnFridayOnEvenWeeks":
            continue  # computed golden, see even_weeks()
        for i, args in enumerate(calls(body)):
            vals = [ev(a, {}) for a in args]
            rule, (_, dt), limit, golden = vals[:4]
            adv, tz = None, "UTC"
            for extra in vals[4:]:
                if isinstance(extra, tuple) and extra[0] == "TZ":
                    tz = extra[1]
                elif extra is not None:
                    adv = extra[1]
            lines = unfold(rule)
            notes = []
            if lines[0].startswith("EXRULE:"):
                notes.append("Upstream passes this EXRULE text to new RRule() and iterates it as a")
                notes.append("generator; RRuleIteratorImpl has no notion of exclusion. Kept as RRULE.")
                lines = ["RRULE:" + lines[0][len("EXRULE:"):]] + lines[1:]
            ref = f"RRuleIteratorImplTest.java#{meth}" + (f"[{i}]" if len(list(calls(body))) > 1 else "")
            emit(out, ref, tz, dt, lines, limit, golden,
                 advance_to=utc_advance(adv) if adv else None, notes=notes)
            n += 1
            if adv is None:
                # The 6-arg helper re-runs itself with advanceTo = dtStart.
                emit(out, ref + " advanceTo=dtStart", tz, dt, lines, limit, golden,
                     advance_to=utc_advance(dt))
                n += 1
    return n


def even_weeks(out):
    # testMonthsThatStartOrEndOnFridayOnEvenWeeks computes its golden in Java:
    # keep the candidates whose TimeUtils.daysBetween(candidate, dtStart) % 14 == 0.
    start = date(1994, 6, 3)
    cands = ["19940701", "19940930", "19950331", "19950630", "19950901", "19951201"]
    keep = [c for c in cands
            if (date(int(c[:4]), int(c[4:6]), int(c[6:])) - start).days % 14 == 0]
    golden = ",".join(keep)
    rule = ["RRULE:FREQ=WEEKLY;INTERVAL=2;BYMONTHDAY=1,-1;BYDAY=FR;COUNT=3"]
    note = ["Golden computed by the upstream test body (candidates whose day offset from",
            "DTSTART is a multiple of 14); the extractor replays that computation."]
    ref = "RRuleIteratorImplTest.java#testMonthsThatStartOrEndOnFridayOnEvenWeeks"
    emit(out, ref, "UTC", "19940603", rule, 8, golden, notes=note)
    emit(out, ref + " advanceTo=dtStart", "UTC", "19940603", rule, 8, golden, advance_to="19940603")
    return 2


# Tests that assert on a hand-written iterator loop rather than the helper.
HAND_RRULE = [
    ("testNextCalledWithoutHasNext", "RRULE:FREQ=DAILY", "20000101", 3,
     "20000101,20000102,20000103", "any",
     "Upstream calls next() three times without hasNext(); it asserts nothing about later instances."),
    ("testNoInstancesGenerated", "RRULE:FREQ=DAILY;UNTIL=19990101", "20000101", 1, "", "no",
     "Upstream asserts hasNext() is false (and next() returns null)."),
    ("testNoInstancesGenerated2", "RRULE:FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=30", "20000101", 1, "", "no",
     "Upstream asserts hasNext() is false."),
    ("testNoInstancesGenerated3", "RRULE:FREQ=YEARLY;INTERVAL=4;BYYEARDAY=366", "20010101", 1, "", "no",
     "Upstream asserts hasNext() is false."),
]


def compound_cases(src, out):
    n = 0
    for meth, body in methods(src):
        cs = list(calls(body))
        for i, args in enumerate(cs):
            if len(args) != 6:
                continue  # testIdenticalInclusionsAndExclusionsNoDtstartPrivilege, see README
            rdata, (_, dt), (_, tz), limit, adv, golden = [ev(a, {}) for a in args]
            ref = f"CompoundIteratorImplTest.java#{meth}" + (f"[{i}]" if len(cs) > 1 else "")
            emit(out, ref, tz, dt, unfold(rdata), limit, golden,
                 advance_to=utc_advance(adv[1]) if adv else None, dtstart_instance="always")
            n += 1
    # testMultipleChecksDontChange: hand loop calling hasNext() twice per step, limit 50.
    emit(out, "CompoundIteratorImplTest.java#testMultipleChecksDontChange", "America/Los_Angeles",
         "20060409", ["RRULE:FREQ=WEEKLY;BYDAY=TH;COUNT=3"], 50, "20060409,20060413,20060420,20060427",
         dtstart_instance="always",
         notes=["Upstream loop calls hasNext() twice per step to check it is idempotent."])
    return n + 1


def main():
    root = Path(sys.argv[1])
    commit = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    out = [
        f"# Extracted from https://github.com/gafter/google-rfc-2445/tree/{commit}/{ITER}",
        "#   RRuleIteratorImplTest.java and CompoundIteratorImplTest.java",
        "# (mirror of code.google.com/p/google-rfc-2445) by extract/rfc2445.py. Do not edit by hand.",
        "# SPDX-FileCopyrightText: 2006 Google Inc.",
        "# SPDX-License-Identifier: Apache-2.0",
        "# (see LICENSES/Apache-2.0.txt). Format: see README.md.",
        "#",
        "# Upstream prints DATE-TIME instances as UTC without a trailing Z (INSTANCES-AS:utc).",
        "# ADVANCE-TO is upstream's advanceTo(), which takes a UTC value.",
    ]
    rr = strip_comments((root / ITER / "RRuleIteratorImplTest.java").read_text())
    n = rrule_iter_cases(rr, out)
    n += even_weeks(out)
    for meth, rule, dt, limit, golden, more, note in HAND_RRULE:
        emit(out, f"RRuleIteratorImplTest.java#{meth}", "UTC", dt, [rule], limit, golden,
             more=more, notes=[note])
        n += 1
    ci = strip_comments((root / ITER / "CompoundIteratorImplTest.java").read_text())
    n += compound_cases(ci, out)
    print("\n".join(out))
    print(f"{n} cases", file=sys.stderr)


if __name__ == "__main__":
    main()
