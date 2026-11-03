//! Supplied report/capture bytes only. No retrieval or origin assertion.
use super::{wire::*, ByteError, ByteResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
fn ids(x: &Value, max: usize, min: usize) -> ByteResult<&[Value]> {
    let xs = list(x, max, min)?;
    for x in xs {
        id(x)?;
    }
    sorted(xs, None)?;
    Ok(xs)
}
fn uri_chars(x: &str, extra: &[u8]) -> bool {
    let b = x.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() || !b[i + 1].is_ascii_hexdigit() || !b[i + 2].is_ascii_hexdigit() {
                return false;
            }
            i += 3;
        } else {
            if !b[i].is_ascii_alphanumeric()
                && !b"._~!$&'()*+,;=-".contains(&b[i])
                && !extra.contains(&b[i])
            {
                return false;
            }
            i += 1;
        }
    }
    true
}
fn uri(x: &Value) -> ByteResult<()> {
    let x = text(x, 2048, 1)?;
    ensure(
        x.bytes().all(|b| (33..=126).contains(&b)) && !x.contains('\\'),
        "ASCII URI",
    )?;
    let (scheme, rest) = x.split_once("://").ok_or(ByteError("absolute URI"))?;
    ensure(
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"),
        "URI scheme",
    )?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    ensure(
        !authority.is_empty() && !authority.contains('@'),
        "URI authority",
    )?;
    let port = if let Some(bracket) = authority.strip_prefix('[') {
        let end = bracket.find(']').ok_or(ByteError("IP literal"))?;
        let (host, rest) = bracket.split_at(end);
        let rest = &rest[1..];
        let future = host
            .get(1..)
            .and_then(|s| s.split_once('.'))
            .is_some_and(|(v, tail)| {
                host.starts_with(['v', 'V'])
                    && !v.is_empty()
                    && v.bytes().all(|b| b.is_ascii_hexdigit())
                    && !tail.is_empty()
                    && tail
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._~!$&'()*+,;=:".contains(&b))
            });
        ensure(
            !host.contains('%') && (host.parse::<std::net::Ipv6Addr>().is_ok() || future),
            "IP literal",
        )?;
        if rest.is_empty() {
            None
        } else {
            Some(rest.strip_prefix(':').ok_or(ByteError("IP port"))?)
        }
    } else {
        let (host, port) = match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        };
        ensure(!host.is_empty() && uri_chars(host, b""), "URI host")?;
        port
    };
    if let Some(port) = port {
        ensure(
            !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()),
            "URI port",
        )?;
        // Leading zeros are legal; avoid bounded-parser rejection of a long zero prefix.
        let significant = port.trim_start_matches('0');
        ensure(
            significant.is_empty() || significant.parse::<u16>().is_ok(),
            "URI port range",
        )?;
    }
    let (path_query, fragment) = tail.split_once('#').map_or((tail, ""), |(a, b)| (a, b));
    let (path, query) = path_query
        .split_once('?')
        .map_or((path_query, ""), |(a, b)| (a, b));
    ensure(
        (path.is_empty() || path.starts_with('/'))
            && uri_chars(path, b":@/")
            && uri_chars(query, b":@/?")
            && uri_chars(fragment, b":@/?"),
        "URI tail",
    )
}
fn time(x: &Value) -> ByteResult<()> {
    let x = text(x, 35, 1)?;
    ensure(x.is_ascii(), "UTC timestamp")?;
    let b = x.as_bytes();
    ensure(
        b.len() >= 20
            && b[4] == b'-'
            && b[7] == b'-'
            && b[10] == b'T'
            && b[13] == b':'
            && b[16] == b':',
        "UTC timestamp",
    )?;
    let n = |start: usize, end: usize| -> ByteResult<u32> {
        let v = &x[start..end];
        ensure(v.bytes().all(|b| b.is_ascii_digit()), "UTC digits")?;
        v.parse().map_err(|_| ByteError("UTC digits"))
    };
    let (year, month, day, hour, minute, second) = (
        n(0, 4)?,
        n(5, 7)?,
        n(8, 10)?,
        n(11, 13)?,
        n(14, 16)?,
        n(17, 19)?,
    );
    ensure(
        chrono::NaiveDate::from_ymd_opt(year as i32, month, day).is_some()
            && hour <= 23
            && minute <= 59
            && second <= 60,
        "UTC date",
    )?;
    let suffix = &x[19..];
    let zone = if let Some(frac) = suffix.strip_prefix('.') {
        let end = frac.find(['Z', '+']).ok_or(ByteError("UTC fraction"))?;
        let (digits, zone) = frac.split_at(end);
        ensure(
            (1..=9).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()),
            "UTC fraction",
        )?;
        zone
    } else {
        suffix
    };
    ensure(zone == "Z" || zone == "+00:00", "UTC offset")?;
    if second == 60 {
        const LEAPS:&str="1972-06-30 1972-12-31 1973-12-31 1974-12-31 1975-12-31 1976-12-31 1977-12-31 1978-12-31 1979-12-31 1981-06-30 1982-06-30 1983-06-30 1985-06-30 1987-12-31 1989-12-31 1990-12-31 1992-06-30 1993-06-30 1994-06-30 1995-12-31 1997-06-30 1998-12-31 2005-12-31 2008-12-31 2012-06-30 2015-06-30 2016-12-31";
        ensure(
            hour == 23 && minute == 59 && LEAPS.split_whitespace().any(|d| d == &x[..10]),
            "known leap second",
        )?;
    }
    Ok(())
}
fn reference(r: &Value, profile: &Value) -> ByteResult<()> {
    id(&r["id"])?;
    digest(&r["excerptDigest"])?;
    match s(&r["kind"])? {
        "file" => {
            exact(r, "kind id path revision location excerptDigest")?;
            let path = text(&r["path"], 2048, 1)?;
            text(&r["revision"], 2048, 1)?;
            text(&r["location"], 2048, 1)?;
            let drive =
                path.as_bytes().get(1) == Some(&b':') && path.as_bytes()[0].is_ascii_alphabetic();
            ensure(
                !path.contains(['\\', '\0'])
                    && !drive
                    && path
                        .split('/')
                        .all(|p| !p.is_empty() && p != "." && p != ".."),
                "root-relative path",
            )?;
        }
        "url" => {
            exact(r, "kind id url retrievedAt excerptDigest")?;
            uri(&r["url"])?;
            time(&r["retrievedAt"])?;
        }
        _ => return Err(ByteError("reference kind")),
    }
    size(r, 1024, "reference size")?;
    let captures = profile["captures"]
        .as_array()
        .ok_or(ByteError("captures"))?;
    let mut matching = captures.iter().filter(|c| c["reference"] == *r);
    let capture = matching.next().ok_or(ByteError("capture absent"))?;
    ensure(matching.next().is_none(), "capture uniqueness")?;
    let encoded = text(&capture["bytesBase64"], 2732, 0)?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| ByteError("capture base64"))?;
    ensure(
        STANDARD.encode(&bytes) == encoded
            && (1..=2048).contains(&bytes.len())
            && hash(bytes) == s(&r["excerptDigest"])?,
        "capture bytes",
    )
}
pub(super) fn validate(source: &Value, r: &Value, profile: &Value) -> ByteResult<()> {
    exact(r,"schemaVersion reportId reportVersion runId planId planRevision taskId reservationId childSessionId configurationDigest sidecarGeneration launchGeneration questionIds findings uncertainties blockers evidenceReferences recommendations limitations author contentDigest")?;
    ensure(r["schemaVersion"] == 1, "report version")?;
    id(&r["reportId"])?;
    number(&r["reportVersion"], 1, 8)?;
    let stream = &profile["stream"];
    exact(stream,"workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest")?;
    for k in [
        "workspaceId",
        "runId",
        "planId",
        "taskId",
        "reservationId",
        "childSessionId",
    ] {
        id(&stream[k])?;
    }
    number(&stream["planRevision"], 1, 64)?;
    digest(&stream["configurationDigest"])?;
    let mut projected = pick(
        r,
        "runId planId planRevision taskId reservationId childSessionId configurationDigest",
    );
    projected["workspaceId"] = stream["workspaceId"].clone();
    same(&projected, stream, "report stream")?;
    exact(&profile["lineage"], "sidecarGeneration launchGeneration")?;
    for k in ["sidecarGeneration", "launchGeneration"] {
        digest(&r[k])?;
        same(&r[k], &profile["lineage"][k], "report lineage")?;
    }
    let questions = ids(&r["questionIds"], 32, 1)?;
    same(
        &r["questionIds"],
        &profile["questionIds"],
        "report questions",
    )?;
    exact(&r["author"], "kind")?;
    tags(&r["author"]["kind"], &["child", "user"])?;
    let refs = list(&r["evidenceReferences"], 64, 0)?;
    sorted(refs, Some("id"))?;
    for r in refs {
        reference(r, profile)?;
    }
    for f in list(&r["findings"], 32, 0)? {
        exact(f, "questionIds answer evidenceReferenceIds classification")?;
        for q in ids(&f["questionIds"], 32, 1)? {
            ensure(questions.contains(q), "finding question")?;
        }
        text(&f["answer"], 2048, 1)?;
        let evidence = ids(&f["evidenceReferenceIds"], 64, 0)?;
        for id in evidence {
            ensure(refs.iter().any(|r| r["id"] == *id), "finding evidence")?;
        }
        tags(
            &f["classification"],
            &["established", "inferred", "unverified"],
        )?;
        ensure(
            f["classification"] != "established" || !evidence.is_empty(),
            "established evidence",
        )?;
        size(f, 2048, "finding size")?;
    }
    for k in [
        "uncertainties",
        "blockers",
        "recommendations",
        "limitations",
    ] {
        for t in list(&r[k], 16, 0)? {
            text(t, 2048, 1)?;
        }
    }
    ensure(
        list(&r["uncertainties"], 16, 0)?.len() + list(&r["blockers"], 16, 0)?.len() <= 16
            && list(&r["recommendations"], 16, 0)?.len() + list(&r["limitations"], 16, 0)?.len()
                <= 16,
        "report aggregate",
    )?;
    sealed(r, "contentDigest", "orkworks.research-report.v1")?;
    size(r, 32768, "report size")?;
    let mut identity=pick(r,"runId planId planRevision taskId reservationId childSessionId configurationDigest sidecarGeneration launchGeneration reportId reportVersion");
    identity["workspaceId"] = stream["workspaceId"].clone();
    same(
        source,
        &json!({"schemaVersion":1,"sourceKind":"research-report","reportIdentity":identity,"sizeBytes":canonical(r)?.len(),"contentDigest":r["contentDigest"]}),
        "report source",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> Value {
        serde_json::from_str(include_str!(
            "../../../../../docs/validation/child-source-vectors.json"
        ))
        .unwrap()
    }
    fn check(mut r: Value, p: &Value) -> ByteResult<()> {
        let body = r.as_object_mut().unwrap();
        body.remove("contentDigest");
        let d = domain("orkworks.research-report.v1", &r)?;
        r["contentDigest"] = json!(d);
        let mut identity=pick(&r,"runId planId planRevision taskId reservationId childSessionId configurationDigest sidecarGeneration launchGeneration reportId reportVersion");
        identity["workspaceId"] = p["stream"]["workspaceId"].clone();
        validate(
            &json!({"schemaVersion":1,"sourceKind":"research-report","reportIdentity":identity,"sizeBytes":canonical(&r)?.len(),"contentDigest":r["contentDigest"]}),
            &r,
            p,
        )
    }
    #[test]
    fn child_sources_report_semantics_survive_rehashing() {
        let p = packet();
        let mut report = p["snapshot"]["evidence"][0]["report"].clone();
        let mut profile = p["reportContext"]["profile"].clone();
        let reference = json!({"kind":"file","id":"ref","path":"a/b","revision":"r","location":"l","excerptDigest":hash(b"x")});
        report["evidenceReferences"] = json!([reference]);
        profile["captures"] = json!([{"reference":reference,"bytesBase64":"eA=="}]);
        let report = &report;
        let profile = &profile;
        assert!(check(report.clone(), profile).is_ok());
        let mutations: Vec<(&str, fn(&mut Value))> = vec![
            ("report version", |r| r["reportVersion"] = json!(9)),
            ("stream", |r| r["reservationId"] = json!("other")),
            ("lineage", |r| r["launchGeneration"] = json!("0".repeat(64))),
            ("question closure", |r| r["questionIds"] = json!(["other"])),
            ("author", |r| r["author"] = json!({"kind":"agent"})),
            ("established without evidence", |r| {
                r["findings"][0]["classification"] = json!("established");
                r["findings"][0]["evidenceReferenceIds"] = json!([]);
            }),
            ("foreign finding question", |r| {
                r["findings"][0]["questionIds"] = json!(["other"])
            }),
            ("foreign finding reference", |r| {
                r["findings"][0]["evidenceReferenceIds"] = json!(["other"])
            }),
            ("finding whole size", |r| {
                r["findings"][0]["answer"] = json!("x".repeat(2048))
            }),
            ("duplicate reference", |r| {
                let a = r["evidenceReferences"][0].clone();
                r["evidenceReferences"].as_array_mut().unwrap().push(a);
            }),
            ("uncertainty aggregate", |r| {
                r["uncertainties"] = json!(vec!["u"; 16]);
                r["blockers"] = json!(["b"]);
            }),
            ("recommendation aggregate", |r| {
                r["recommendations"] = json!(vec!["r"; 16]);
                r["limitations"] = json!(["l"]);
            }),
            ("unknown field", |r| r["approved"] = json!(true)),
        ];
        for (name, mutate) in mutations {
            let mut r = report.clone();
            mutate(&mut r);
            assert!(check(r, profile).is_err(), "{name}");
        }
        let mut duplicate = profile.clone();
        let c = duplicate["captures"][0].clone();
        duplicate["captures"].as_array_mut().unwrap().push(c);
        assert!(check(report.clone(), &duplicate).is_err());
        let mut drift = profile.clone();
        drift["captures"][0]["bytesBase64"] = json!("eQ==");
        assert!(check(report.clone(), &drift).is_err());
    }
    #[test]
    fn child_sources_report_reference_grammar_is_pure_and_exact() {
        for value in ["https://example.test/a%20b?q=x#ok","HTTP://[::1]:65535/","https://[v1.a:b]/","https://ex%41mple.test:0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000/"] {assert!(uri(&json!(value)).is_ok(),"{value}");}
        for value in [
            "file:///x",
            "https://user@host/",
            "https:///x",
            "https://[fe80::1%25lo]/",
            "https://::1/",
            "https://host:/",
            "https://host:65536/",
            "https://host/%GG",
            "https://host/é",
            "https://host/\\x",
            "https://host/#x#y",
            "https://[v1.]/",
        ] {
            assert!(uri(&json!(value)).is_err(), "{value}");
        }
        for value in [
            "0000-02-29T00:00:00Z",
            "2024-02-29T23:59:59.123456789+00:00",
            "2016-12-31T23:59:60Z",
        ] {
            assert!(time(&json!(value)).is_ok(), "{value}");
        }
        for value in [
            "2023-02-29T00:00:00Z",
            "2024-01-01T24:00:00Z",
            "2024-01-01T23:59:60Z",
            "2016-12-31T22:59:60Z",
            "2016-12-31T23:59:60.1234567890Z",
            "2024-01-01T00:00:00+01:00",
            "2024-01-01T00:00:00.Z",
            "é",
        ] {
            assert!(time(&json!(value)).is_err(), "{value}");
        }
        for path in ["../x", "/x", "a//b", "a/./b", "C:/x", "a\\b", "a\0b"] {
            let r = json!({"kind":"file","id":"ref","path":path,"revision":"r","location":"l","excerptDigest":hash(b"x")});
            let p = json!({"captures":[{"reference":r,"bytesBase64":"eA=="}]});
            assert!(reference(&r, &p).is_err(), "{path}");
        }
        let r = json!({"kind":"file","id":"ref","path":"a/b","revision":"r","location":"l","excerptDigest":hash([0xff])});
        let p = json!({"captures":[{"reference":r,"bytesBase64":"/w=="}]});
        assert!(
            reference(&r, &p).is_ok(),
            "opaque capture bytes need not be UTF-8"
        );
    }
}
