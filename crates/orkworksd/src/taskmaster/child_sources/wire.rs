// Only owner-neutral pure byte checks are shared with the unchanged root entry.
use super::{ByteError, ByteResult};
pub(super) use crate::taskmaster::root_bootstrap::wire::*;
use serde_json::{json, Value};
pub(super) fn same(a: &Value, b: &Value, why: &'static str) -> ByteResult<()> {
    ensure(a == b, why)
}
pub(super) fn size(x: &Value, max: usize, why: &'static str) -> ByteResult<()> {
    ensure(canonical(x)?.len() <= max, why)
}
pub(super) fn pick(x: &Value, fields: &str) -> Value {
    Value::Object(
        fields
            .split_whitespace()
            .map(|k| (k.into(), x[k].clone()))
            .collect(),
    )
}
pub(super) fn domain(name: &str, x: &Value) -> ByteResult<String> {
    let mut b = name.as_bytes().to_vec();
    b.push(b'\n');
    b.extend(canonical(x)?);
    Ok(hash(b))
}
pub(super) fn add(total: usize, n: usize, max: usize, why: &'static str) -> ByteResult<usize> {
    total
        .checked_add(n)
        .filter(|n| *n <= max)
        .ok_or(ByteError(why))
}
pub(super) fn task(c: &Value, run: &Value) -> Value {
    let mut t = pick(c, "workspaceId planId planRevision taskId");
    t["runId"] = run["runId"].clone();
    t
}
pub(super) fn identity(x: &Value) -> ByteResult<()> {
    exact(x,"kind workspaceId runId planId planRevision taskId taskVersion reservationId parentSessionId childSessionId configurationId configurationDigest")?;
    ensure(x["kind"] == "child", "child identity")?;
    for k in [
        "workspaceId",
        "runId",
        "planId",
        "taskId",
        "reservationId",
        "parentSessionId",
        "childSessionId",
        "configurationId",
    ] {
        id(&x[k])?;
    }
    number(&x["planRevision"], 1, 64)?;
    number(&x["taskVersion"], 1, 2147483647)?;
    digest(&x["configurationDigest"])?;
    Ok(())
}
pub(super) fn criteria(xs: &Value, ordered: bool) -> ByteResult<()> {
    let xs = list(xs, 32, 1)?;
    let mut ids = std::collections::BTreeSet::new();
    let mut required = false;
    for x in xs {
        exact(x, "id requirement description")?;
        ensure(ids.insert(id(&x["id"])?), "criterion id")?;
        tags(&x["requirement"], &["required", "optional"])?;
        required |= x["requirement"] == "required";
        text(&x["description"], 2048, 1)?;
    }
    ensure(required, "required criterion")?;
    if ordered {
        sorted(xs, Some("id"))?;
    }
    Ok(())
}
pub(super) fn commit(x: &Value) -> ByteResult<()> {
    exact(x, "storageEpoch commitSequence changeDigest")?;
    digest(&x["storageEpoch"])?;
    digest(&x["changeDigest"])?;
    let n = s(&x["commitSequence"])?;
    ensure(
        !n.is_empty()
            && !n.starts_with('0')
            && n.len() <= 20
            && n.bytes().all(|b| b.is_ascii_digit())
            && n.parse::<u64>().is_ok(),
        "commit Counter",
    )
}
pub(super) fn plan_ref(t: &Value, p: &Value) -> Value {
    let mut r = pick(t, "workspaceId runId planId planRevision");
    r["planDefinitionDigest"] = p["planDefinitionDigest"].clone();
    r
}
pub(super) fn reservation_ref(t: &Value, r: &Value) -> Value {
    json!({"task":t,"reservationId":r["reservationId"],"reservationDigest":r["reservationDigest"]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_sources_raw_byte_ceilings_are_inclusive_and_checked() {
        for max in [4096, 16384, 32768, 65536, 262144, 1048576, 2097152] {
            let value = json!("x".repeat(max - 2));
            let bytes = canonical(&value).unwrap();
            assert_eq!(bytes.len(), max);
            assert_eq!(decode(&bytes, max).unwrap(), value);
            assert!(decode(&canonical(&json!("x".repeat(max - 1))).unwrap(), max).is_err());
        }
        assert_eq!(
            add(usize::MAX, 1, usize::MAX, "overflow"),
            Err(ByteError("overflow"))
        );
    }
}
