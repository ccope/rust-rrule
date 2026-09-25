#!/usr/bin/env python3
"""Regenerate ../ical4j_recur_tests.txt from an ical4j checkout.

Usage: ical4j.py /path/to/ical4j > ../ical4j_recur_tests.txt

RecurSpec.groovy: the Spock `where:` tables of the expansion features are read
mechanically (table_specs() maps each feature's row to a case); the
non-table features and the RecurTest.java cases are transcribed in hand_cases()
with the reason for each. Parse-only and validation features are not extracted
(see README.md).
"""
import ast
import re
import subprocess
import sys
from pathlib import Path

SPEC = "src/test/groovy/net/fortuna/ical4j/model/RecurSpec.groovy"
RECUR_TEST = "src/test/java/net/fortuna/ical4j/model/RecurTest.java"


def features(src):
    for m in re.finditer(r"^\s*def (['\"])(.+?)\1\(\)\s*\{", src, re.M):
        i, depth = m.end(), 1
        while depth:
            depth += {"{": 1, "}": -1}.get(src[i], 0)
            i += 1
        yield m.group(2), src[m.end() : i - 1]


def where_rows(body):
    lines = body.split("where:", 1)[1].splitlines()
    rows, header = [], None
    for line in lines:
        s = line.strip()
        if not s or s.startswith("//"):
            if header and rows and not s:
                break
            continue
        if "|" not in s:
            if header:
                break
            continue
        cells = [c.strip() for c in re.split(r"\|\|?", s)]
        if header is None:
            header = cells
        else:
            rows.append({h: ast.literal_eval(c) for h, c in zip(header, cells)})
    return rows


def groovy_list(text):
    """The `expected << [ ... ]` column of 'verify recurrence rule'."""
    start = text.index("expected << [") + len("expected << ")
    i, depth = start, 0
    while True:
        depth += {"[": 1, "]": -1}.get(text[i], 0)
        i += 1
        if depth == 0:
            break
    inner = text[start + 1 : i - 1]
    items, depth, cur = [], 0, ""
    for c in inner:
        depth += {"[": 1, "(": 1, "{": 1, "]": -1, ")": -1, "}": -1}.get(c, 0)
        if c == "," and depth == 0:
            items.append(cur.strip())
            cur = ""
        else:
            cur += c
    items.append(cur.strip())
    out = []
    for it in items:
        m = re.fullmatch(r'\((\d+)\.\.(\d+)\)\.collect \{ format\("(\d*)%02d", it\)\}', it)
        if m:
            out.append([f"{m.group(3)}{d:02d}" for d in range(int(m.group(1)), int(m.group(2)) + 1)])
        else:
            out.append(ast.literal_eval(it))
    return out


def iso_local(s):
    """'2025-03-05T05:00:00' -> '20250305T050000'."""
    d, t = s.split("T")
    t = (t + ":00")[:8] if len(t) == 5 else t
    return d.replace("-", "") + "T" + t.replace(":", "")


def zdt(s):
    """ZonedDateTime.toString(): '2025-03-05T05:00-08:00[America/Los_Angeles]' -> local-offset."""
    m = re.fullmatch(r"(\d{4}-\d\d-\d\dT\d\d:\d\d(?::\d\d)?)([+-]\d\d):(\d\d)\[.*\]", s)
    return iso_local(m.group(1)) + m.group(2) + m.group(3)


class Out:
    def __init__(self):
        self.lines, self.n = [], 0

    def case(self, ref, dtstart, rule, *, tz="UTC", window=None, after=None, limit=None,
             instances=None, more=None, count=None, nth=None, same_as=None, fmt="local",
             notes=()):
        L = self.lines
        L.append("")
        L.extend(f"# {n}" for n in notes)
        L.append(f"CASE:{ref}")
        L.append(f"TZ:{tz}")
        L.append(f"DTSTART:{dtstart}")
        L.append("DTSTART-INSTANCE:if-matches")
        L.append(f"RRULE:{rule}")
        if window:
            L.append(f"WINDOW:{window[0]}/{window[1]}")
        if after:
            L.append(f"AFTER:{after}")
        if limit is not None:
            L.append(f"LIMIT:{limit}")
        L.append(f"INSTANCES-AS:{fmt}")
        if instances is not None:
            L.append(f"INSTANCES:{','.join(instances)}")
        if more:
            L.append(f"MORE:{more}")
        if count is not None:
            L.append(f"COUNT:{count}")
        if nth is not None:
            L.append(f"NTH:{nth[0]}={nth[1]}")
        if same_as:
            L.append(f"SAME-AS:{same_as}")
        self.n += 1


def spec_ref(title, i, rows):
    return f"RecurSpec.groovy#{title}" + (f" [{i}]" if len(rows) > 1 else "")



def table_specs(src, out):
    feats = dict(features(src))

    t = "verify recurrence rule: #rule"
    rows, exp = where_rows(feats[t]), groovy_list(feats[t])
    assert len(rows) == len(exp)
    for i, (r, x) in enumerate(zip(rows, exp)):
        out.case(spec_ref(t, i, rows), r["start"], r["rule"], window=(r["start"], r["end"]), instances=x)

    t = "verify byweekno recurrence rules without byday: #rule wkst: #wkst"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        out.case(spec_ref(t, i, rows), r["start"], f"FREQ=YEARLY;BYWEEKNO={r['rule']};WKST={r['wkst']}",
                 window=(r["start"], r["end"]), instances=r["expected"])

    t = "verify byweekno recurrence rules: #rule wkst: #wkst byday: #byday"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        out.case(spec_ref(t, i, rows), r["start"],
                 f"FREQ=YEARLY;BYWEEKNO={r['rule']};WKST={r['wkst']};BYDAY={r['byday']}",
                 window=(r["start"], r["end"]), instances=r["expected"])

    t = "verify monthly bymonthday recurrence rules: #rule #year"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        y = r["year"]
        out.case(spec_ref(t, i, rows), y + r["start"], f"FREQ=MONTHLY;BYMONTHDAY={r['rule']}",
                 window=(y + r["start"], y + r["end"]), instances=[y + d for d in r["expected"]])

    for t, tmpl in [("verify yearly bymonthday recurrence rules: #rule #start", "FREQ=YEARLY;BYMONTHDAY={}"),
                    ("verify byyearday recurrence rules: #rule", "FREQ=YEARLY;BYYEARDAY={}")]:
        rows = where_rows(feats[t])
        for i, r in enumerate(rows):
            out.case(spec_ref(t, i, rows), r["start"], tmpl.format(r["rule"]),
                     window=(r["start"], r["end"]), instances=r["expected"])

    for t, note in [
        ("verify recurrence rule in different locales: #rule", "Upstream runs this with Locale.FRANCE."),
        ("verify recurrence rule with a specified interval: #rule", None),
        ("verify recurrence rule with a specified WKST: #rule", None),
        ("verify recurrence rule in different locales with a specified interval: #rule",
         "Upstream runs this with Locale.FRANCE."),
    ]:
        rows = where_rows(feats[t])
        for i, r in enumerate(rows):
            out.case(spec_ref(t, i, rows), r["start"], r["rule"], window=(r["start"], r["end"]),
                     instances=r["expected"], notes=[note] if note else ())

    t = "verify recurrence rule in different system timezones: #systemTimezone"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        out.case(spec_ref(t, i, rows), r["start"], r["rule"], tz="Europe/Berlin",
                 window=(r["start"], r["end"]), instances=r["expected"],
                 notes=[f"Upstream JVM default time zone: {r['systemTimezone']} (values are Europe/Berlin)."])

    t = "duplicate candidates do not consume count positions: #repeated"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        out.case(spec_ref(t, i, rows), "20080115T090000Z", r["repeated"] + ";COUNT=12",
                 window=("20080115T090000Z", "20280115T090000Z"), count=12,
                 same_as=r["canonical"] + ";COUNT=12", fmt="utc-z",
                 notes=["Expected = the canonical rule's instances (upstream also checks getNextDate)."])

    t = "test Recur.getNextDate() with different recurrence rules"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        out.case(spec_ref(t, i, rows), r["seed"], r["rule"], after=r["start"], limit=1,
                 instances=[r["expectedDate"]], more="any",
                 notes=["getNextDate(seed, start): first instance strictly after start."])

    t = "verify yearly recurrence with UTC start date in system timezone: #systemTimezone"
    zones = re.search(r"systemTimezone << (\[.*?\])", feats[t]).group(1)
    rows = [{"systemTimezone": z} for z in ast.literal_eval(zones)]
    for i, r in enumerate(rows):
        out.case(spec_ref(t, i, rows), "20250302T000000Z", "FREQ=YEARLY;INTERVAL=1",
                 window=("20250101T000000Z", "20251231T000000Z"), instances=["20250302T000000Z"],
                 fmt="utc-z", notes=[f"Upstream JVM default time zone: {r['systemTimezone']}."])

    t = "test getdates across a DST-change both ways"
    rows = where_rows(feats[t])
    for i, r in enumerate(rows):
        exp = [zdt(v.strip()) for v in r["expectedList"][1:-1].split(", ")]
        out.case(spec_ref(t, i, rows), "20250305T%02d0000" % r["hour"], "FREQ=DAILY;COUNT=10;INTERVAL=1",
                 tz="America/Los_Angeles", window=(iso_local(r["pStart"]), iso_local(r["pEnd"])),
                 instances=exp, fmt="local-offset")


def hand_cases(out):
    """RecurSpec features without a where: table; each transcribed from its body."""
    S = "RecurSpec.groovy#"
    minutely = "FREQ=DAILY;INTERVAL=1;BYDAY=MO;BYHOUR=16;BYMINUTE=37,38;UNTIL=20220519T165900"
    exp = "20220509T163700,20220509T163800,20220516T163700,20220516T163800".split(",")
    built = "Rule text is what upstream's Recur.Builder chain produces (frequency, interval, dayList, hourList, minuteList, until)."
    out.case(S + "test BYDAY with MINUTELY precision", "20220505T143700", minutely,
             window=("20220505T143700", "20220519T145900"), instances=exp, notes=[built])
    out.case(S + "test getdates as stream", "20220505T143700", minutely,
             window=("20220505T143700", "20220519T145900"), instances=exp, notes=[built])
    out.case(S + "test unlike temporals", "20200101T000000", "FREQ=YEARLY;INTERVAL=2;UNTIL=20230101",
             window=("20200101T000000", "20250101T000000"),
             instances=["20200101T000000", "20220101T000000"],
             notes=["DATE UNTIL against a DATE-TIME seed."])
    big = ("20520101T163700, 20520101T163800, 20520108T163700, 20520108T163800, 20520115T163700, "
           "20520115T163800, 20520122T163700, 20520122T163800, 20520129T163700, 20520129T163800, "
           "20520205T163700, 20520205T163800, 20520212T163700, 20520212T163800, 20520219T163700, "
           "20520219T163800, 20520226T163700, 20520226T163800, 20520304T163700, 20520304T163800, "
           "20520311T163700, 20520311T163800, 20520318T163700, 20520318T163800, 20520325T163700, "
           "20520325T163800, 20520401T163700, 20520401T163800, 20520408T163700, 20520408T163800, "
           "20520415T163700, 20520415T163800, 20520422T163700, 20520422T163800, 20520429T163700, "
           "20520429T163800, 20520506T163700, 20520506T163800, 20520513T163700, 20520513T163800")
    out.case(S + "test getdates as stream for a large date range", "20220505T143700",
             minutely.replace("UNTIL=2022", "UNTIL=2052"), window=("20520101T000000", "20520519T145900"),
             instances=big.split(", "),
             notes=[built, "Upstream window is 20220505T143700/20520519T145900 filtered to YEAR > 2051;",
                    "the window start here is that filter (every instance is at 16:37 or 16:38)."])
    out.case(S + "test getdates as string with seed far in the past", "20200101T000000", "FREQ=SECONDLY;INTERVAL=1",
             window=("20260101T000000", "20260131T235959"), count=86400 * 31,
             notes=["Upstream: @Timeout(5), count == 86400 * 31."])
    out.case(S + "test secondly with multiple filtering rules", "20200101T000000",
             "FREQ=SECONDLY;INTERVAL=1;BYMONTH=1;BYMONTHDAY=31;BYHOUR=23;BYMINUTE=59;BYSECOND=0",
             window=("20260101T000000", "20260131T235959"), count=1,
             notes=["Upstream: @Timeout(5) with maxincrementcount=50,000,000."])
    out.case(S + "test getdates across a DST-change", "20000201T005900", "FREQ=WEEKLY;INTERVAL=1;BYDAY=FR,SA,SU,MO",
             tz="America/Los_Angeles", window=("20191020T200000Z", "20191110T000000Z"),
             nth=(7, "20191103T005900-0700"), fmt="local-offset",
             notes=["Upstream only asserts list.get(7) == ZonedDateTime.of(2019, 11, 3, 0, 59, 0, 0, LA)."])
    out.case(S + "test conflicting rule execution times, also with high maxincrementcount", "20220101T000000",
             "FREQ=MONTHLY;BYMONTHDAY=1;BYDAY=2SU", window=("20220101T000000", "20240101T000000"),
             instances=[], notes=["Upstream: @Timeout(5) with maxincrementcount=50,000,000."])

    R = "RecurTest.java#"
    disabled = "RecurTest is @Disabled upstream (\"Failed after re-enabling JUnit 3/4 tests\"): not exercised by ical4j's build."
    mel = "Upstream sets the JVM default zone to Australia/Melbourne and parses with ZoneId.systemDefault()."
    flo = "Upstream parses with ZoneId.systemDefault(), which the test does not set; run as floating (UTC)."
    for ref, seed, rule, w0, w1, n, tz, note in [
        ("testGetDatesCount firstFourWeeksOfYear", "20130101T120000", "FREQ=YEARLY;BYWEEKNO=1,2,3,4",
         "20130101T120000", "20130123T120000", 4, "Australia/Melbourne", mel),
        ("testGetDatesCount everySecondWeek (issue 576)", "20220501",
         "FREQ=YEARLY;BYDAY=WE;BYWEEKNO=1,3,5,7,9,11,13,15,17,19,21,23,25,27,29,31,33,35,37,39,41,43,45,47,49,51,53",
         "20220501", "20230501", 27, "UTC",
         "getDates(periodStart, periodEnd): seed = periodStart. UPSTREAM MAY BE WRONG: with ISO/WKST=MO "
         "week numbers the window holds 26 such Wednesdays (odd weeks 19-51 of 2022, 1-17 of 2023)."),
        ("testGetDatesCount threeWeekdays", "20131215T000000", "FREQ=DAILY;COUNT=3;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR",
         "20131215T000000", "20180101T120000", 3, "Australia/Melbourne", mel),
        ("testGetDatesCount firstFourWeeksOfYear 2016", "20131215T000000", "FREQ=YEARLY;BYWEEKNO=1,2,3,4",
         "20160101T120000", "20160123T120000", 3, "Australia/Melbourne", mel),
        ("testGetDatesMaxTime everyThursdayUntil", "20160414T100000",
         "FREQ=WEEKLY;WKST=MO;UNTIL=20160901T230000;INTERVAL=1;BYDAY=TH",
         "20110713T213021", "20230713T213021", 21, "UTC", "Upstream also asserts it runs in under 100ms."),
        ("testGetDatesMaxTime firstSaturdayOfMonth", "20160507T090000", "FREQ=MONTHLY;WKST=MO;INTERVAL=1;BYDAY=1SA",
         "20110713T213022", "20260713T213022", 123, "UTC", "Upstream also asserts it runs in under 100ms."),
        ("testGetDatesMaxTime everyWednesday", "20160427T160000", "FREQ=WEEKLY;WKST=MO;INTERVAL=1;BYDAY=WE",
         "20110713T213022", "20230713T213022", 377, "UTC", "Upstream also asserts it runs in under 100ms."),
        ("testGetDatesMaxTime everyThursday", "20200324T200000", "FREQ=WEEKLY;WKST=MO;INTERVAL=1;BYDAY=TU",
         "20110714T083812", "20230714T083812", 173, "UTC", "Upstream also asserts it runs in under 100ms."),
    ]:
        out.case(R + ref, seed, rule, tz=tz, window=(w0, w1), count=n, notes=[disabled, note])

    nd = "getNextDate(seed, start): first instance strictly after start; empty = upstream expects null."
    for ref, seed, rule, start, exp, extra in [
        ("testGetNextDate everyDay [0]", "20080401", "FREQ=DAILY;COUNT=3", "20080401", "20080402", None),
        ("testGetNextDate everyDay [1]", "20080401", "FREQ=DAILY;COUNT=3", "20080402", "20080403", None),
        ("testGetNextDate everyDay [2]", "20080401", "FREQ=DAILY;COUNT=3", "20080403", None, None),
        ("testGetNextDate weeklyUntil [0]", "20080407T063000", "FREQ=WEEKLY;UNTIL=20080421T063000",
         "20080407T063000", "20080414T063000", flo),
        ("testGetNextDate weeklyUntil [1]", "20080407T063000", "FREQ=WEEKLY;UNTIL=20080421T063000",
         "20080414T063000", "20080421T063000", flo),
        ("testGetNextDate weeklyUntil [2]", "20080407T063000", "FREQ=WEEKLY;UNTIL=20080421T063000",
         "20080421T063000", None, flo),
        ("testGetNextDate everyMonday", "20081212", "FREQ=WEEKLY;BYDAY=MO", "20081211", "20081215",
         "Search start is before the seed."),
        ("testGetNextDate firstSundayOfApril", "20081103T070000", "FREQ=YEARLY;BYMONTH=4;BYDAY=1SU",
         "20081109T210000", "20090405T070000", flo),
        ("testGetNextDate everySecondYearLastDayOfMonth", "20081103T070000",
         "FREQ=YEARLY;COUNT=4;INTERVAL=2;BYMONTH=1,2,3;BYMONTHDAY=-1", "20081109T210000", "20100131T070000", flo),
        ("testGetNextDate lastDayOfFebMarSepOct", "20150701T000000",
         "FREQ=MONTHLY;WKST=MO;INTERVAL=1;BYMONTH=2,3,9,10;BYMONTHDAY=28,29,30,31;BYSETPOS=-1",
         "20150701T000000", "20150930T000000", flo),
        ("testGetNextDate everySecondMonth", "20200229T000000", "FREQ=MONTHLY;BYMONTH=2;INTERVAL=1",
         "20200229T000000", "20240229T000000",
         "Upstream comment says 'should return feb 28 2021' but the asserted value is 2024-02-29."),
        ("testGetNextDate everySecondMonth30th", "20200229T000000", "FREQ=MONTHLY;BYMONTH=2;BYMONTHDAY=30;INTERVAL=1",
         "20200229T000000", None, flo),
        ("testGetNextDate every29th", "20200229T000000", "FREQ=YEARLY;BYMONTHDAY=29;INTERVAL=1",
         "20200229T000000", "20240229T000000",
         "RFC 5545 expands YEARLY;BYMONTHDAY across all months (next: 20200329); ical4j limits it to the seed's month."),
        ("testGetNextDate everyFourthYear", "20200229T000000", "FREQ=YEARLY;INTERVAL=4",
         "20200229T000000", "20240229T000000", flo),
        ("testGetNextDate lastWeekdayOfMonth", "20200531T000000", "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1",
         "20200531T000000", "20200731T000000",
         "UPSTREAM LOOKS WRONG: its own comment says 'should return jun 30 2020' (a Tuesday); asserts 2020-07-31."),
        ("testGetNextDate fifthSundayOfMonth", "20200831T000000", "FREQ=MONTHLY;BYDAY=SU;BYSETPOS=5",
         "20200831T000000", "20210131T000000",
         "UPSTREAM LOOKS WRONG: its own comment says 'should return nov 29 2020' (the 5th Sunday); asserts 2021-01-31."),
    ]:
        notes = [disabled, nd] + ([extra] if extra else [])
        out.case(R + ref, seed, rule, after=start, limit=1, instances=[exp] if exp else [],
                 more="any" if exp else "no", notes=notes)


def main():
    root = Path(sys.argv[1])
    commit = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    out = Out()
    out.lines += [
        f"# Extracted from https://github.com/ical4j/ical4j/tree/{commit}",
        f"#   {SPEC}",
        f"#   {RECUR_TEST} (hand-picked cases)",
        "# by extract/ical4j.py. Do not edit by hand.",
        "# SPDX-FileCopyrightText: 2012 Ben Fortuna",
        "# SPDX-License-Identifier: BSD-3-Clause",
        "# (see LICENSES/BSD-3-Clause.txt). Format: see README.md.",
        "#",
        "# ical4j Recur.getDates(seed, periodStart, periodEnd): DTSTART = seed; WINDOW is",
        "# [periodStart, periodEnd] inclusive at both ends; the seed is an instance only if",
        "# the rule generates it; COUNT counts from the seed, including instances before",
        "# the window. getDates(start, end) uses start as the seed. Floating values",
        "# (LocalDate/LocalDateTime) are run in TZ:UTC; a DATE UNTIL means 00:00 that day.",
    ]
    table_specs((root / SPEC).read_text(), out)
    hand_cases(out)
    print("\n".join(out.lines))
    print(f"{out.n} cases", file=sys.stderr)


if __name__ == "__main__":
    main()
