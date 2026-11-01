use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct HistoryQuery {
    pub(crate) state: Option<String>,
    pub(crate) harness: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) text: String,
    pub(crate) limit: Option<usize>,
    pub(crate) cursor: Option<Cursor>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Cursor {
    epoch: String,
    workspace: String,
    generation: i64,
    digest: String,
    recency: i64,
    id: String,
}
#[derive(Debug, Serialize)]
pub(crate) struct Summary {
    pub(crate) id: String,
    pub(crate) created_at: String,
    pub(crate) revision: String,
    pub(crate) organization_revision: String,
    pub(crate) state: String,
    pub(crate) recency: i64,
    pub(crate) title: String,
    pub(crate) summary: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) harness: String,
    pub(crate) model: String,
    pub(crate) native_id: Option<String>,
    pub(crate) preview_truncated: bool,
}
#[derive(Debug, Serialize)]
pub(crate) struct Page {
    pub(crate) rows: Vec<Summary>,
    pub(crate) cursor: Option<Cursor>,
    pub(crate) has_more: bool,
    pub(crate) generation: i64,
}
const SUMMARY_SELECT:&str = "SELECT s.id,s.created_at,s.revision,coalesce(o.revision,'0'),s.state,s.recency,
 substr(s.title,1,256),substr(s.summary,1,512),substr(s.repository,1,256),substr(s.branch,1,128),
 substr(s.harness,1,64),substr(s.model,1,64),substr(s.native_id,1,128),
 (length(s.title)>256 OR length(coalesce(s.summary,''))>512 OR length(coalesce(s.repository,''))>256
 OR length(coalesce(s.branch,''))>128 OR length(s.harness)>64 OR length(s.model)>64 OR length(coalesce(s.native_id,''))>128)
 FROM sessions s LEFT JOIN organization o ON o.id=s.id AND o.created_at=s.created_at";
fn summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<Summary> {
    Ok(Summary {
        id: row.get(0)?,
        created_at: row.get(1)?,
        revision: row.get(2)?,
        organization_revision: row.get(3)?,
        state: row.get(4)?,
        recency: row.get(5)?,
        title: row.get(6)?,
        summary: row.get(7)?,
        repository: row.get(8)?,
        branch: row.get(9)?,
        harness: row.get(10)?,
        model: row.get(11)?,
        native_id: row.get(12)?,
        preview_truncated: row.get(13)?,
    })
}
fn tokens(text: &str) -> Result<Option<String>> {
    if text.len() > 256
        || text
            .chars()
            .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
    {
        return Err(StoreError::Invalid("invalid metadata search".into()));
    }
    let terms: Vec<_> = text.split_whitespace().collect();
    if terms.len() > 8
        || terms
            .iter()
            .any(|t| matches!(*t, "OR" | "AND" | "NOT" | "NEAR"))
    {
        return Err(StoreError::Invalid("invalid metadata search".into()));
    }
    if terms.is_empty() {
        return Ok(None);
    }
    let last = terms.len() - 1;
    Ok(Some(
        terms
            .into_iter()
            .enumerate()
            .map(|(i, t)| format!("\"{t}\"{}", if i == last { "*" } else { "" }))
            .collect::<Vec<_>>()
            .join(" AND "),
    ))
}
pub(super) struct ReadBudget<'a>(&'a Connection);
impl<'a> ReadBudget<'a> {
    pub(super) fn new(conn: &'a Connection) -> Self {
        let deadline = std::time::Instant::now() + Duration::from_millis(100);
        conn.progress_handler(1000, Some(move || std::time::Instant::now() >= deadline));
        Self(conn)
    }
}
impl Drop for ReadBudget<'_> {
    fn drop(&mut self) {
        self.0.progress_handler(0, None::<fn() -> bool>);
    }
}
impl Database {
    pub(super) fn history(&self, query: HistoryQuery) -> Result<Page> {
        use sha2::{Digest, Sha256};
        let limit = query.limit.unwrap_or(50);
        if !(1..=100).contains(&limit)
            || query
                .state
                .as_deref()
                .is_some_and(|s| !matches!(s, "closed" | "archived" | "all"))
        {
            return Err(StoreError::Invalid("invalid history query".into()));
        }
        for value in [&query.harness, &query.repository, &query.branch]
            .into_iter()
            .flatten()
        {
            if value.len() > 4096 {
                return Err(StoreError::TooLarge);
            }
        }
        let search = tokens(&query.text)?;
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&(
            &query.state,
            &query.harness,
            &query.repository,
            &query.branch,
            &query.text,
        ))?));
        let _budget = ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        let (epoch, workspace, generation): (String, String, i64) = tx.query_row(
            "SELECT epoch,workspace,generation FROM store_header",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let workspace = hex::encode(Sha256::digest(workspace.as_bytes()));
        if let Some(c) = &query.cursor {
            if serde_json::to_vec(c)?.len() > 2048
                || c.epoch != epoch
                || c.workspace != workspace
                || c.generation != generation
                || c.digest != digest
            {
                return Err(StoreError::Conflict);
            }
        }
        let mut sql = format!("{SUMMARY_SELECT} WHERE s.deleted=0");
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        match query.state.as_deref().unwrap_or("all") {
            "closed" | "archived" => {
                sql.push_str(" AND s.state=?");
                values.push(query.state.clone().unwrap_or_default().into());
            }
            _ => sql.push_str(" AND s.state IN ('closed','unknown')"),
        }
        for (column, value) in [
            ("harness", &query.harness),
            ("repository", &query.repository),
            ("branch", &query.branch),
        ] {
            if let Some(value) = value {
                sql.push_str(&format!(" AND s.{column}=?"));
                values.push(value.clone().into());
            }
        }
        if let Some(search) = search {
            sql.push_str(
                " AND s.id IN (SELECT id FROM metadata_search WHERE metadata_search MATCH ?)",
            );
            values.push(search.into());
        }
        if let Some(c) = query.cursor {
            sql.push_str(" AND (s.recency,s.id)<(?,?)");
            values.push(c.recency.into());
            values.push(c.id.into());
        }
        sql.push_str(" ORDER BY s.recency DESC,s.id DESC LIMIT ?");
        values.push(((limit + 1) as i64).into());
        let mut statement = tx.prepare(&sql)?;
        let mut records = statement.query(rusqlite::params_from_iter(values))?;
        let mut page = Page {
            rows: Vec::new(),
            cursor: None,
            has_more: false,
            generation,
        };
        // Reserve envelope/cursor overhead; row identities and cursors are bounded.
        let mut bytes = 4096;
        while let Some(row) = records.next()? {
            let row = summary(row)?;
            let size = serde_json::to_vec(&row)?.len();
            if size > 8192 {
                return Err(StoreError::TooLarge);
            }
            if page.rows.len() == limit || bytes + size + 1 > 256 * 1024 {
                page.has_more = true;
                break;
            }
            bytes += size + 1;
            page.rows.push(row);
        }
        if page.has_more {
            let last = page.rows.last().ok_or(StoreError::TooLarge)?;
            page.cursor = Some(Cursor {
                epoch,
                workspace,
                generation,
                digest,
                recency: last.recency,
                id: last.id.clone(),
            });
            if serde_json::to_vec(&page.cursor)?.len() > 2048 {
                return Err(StoreError::TooLarge);
            }
        }
        drop(records);
        drop(statement);
        tx.commit()?;
        if serde_json::to_vec(&page)?.len() > 256 * 1024 {
            return Err(StoreError::TooLarge);
        }
        Ok(page)
    }
    pub(super) fn live(&self, ids: &[String]) -> Result<Vec<Summary>> {
        if ids.len().saturating_mul(8192).saturating_add(16384) > 32 * 1024 * 1024 {
            return Err(StoreError::TooLarge);
        }
        let _budget = ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        let mut statement =
            tx.prepare(&format!("{SUMMARY_SELECT} WHERE s.deleted=0 AND s.id=?1"))?;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let row = statement
                .query_row([id], summary)
                .optional()?
                .ok_or(StoreError::Missing)?;
            if serde_json::to_vec(&row)?.len() > 8192 {
                return Err(StoreError::TooLarge);
            }
            result.push(row);
        }
        drop(statement);
        tx.commit()?;
        Ok(result)
    }
    pub(super) fn exists(&self, ids: &[String]) -> Result<Vec<String>> {
        if ids.len() > 256 {
            return Err(StoreError::TooLarge);
        }
        let _budget = ReadBudget::new(&self.conn);
        let tx = self.conn.unchecked_transaction()?;
        let mut statement = tx.prepare("SELECT id FROM sessions WHERE id=?1 AND deleted=0")?;
        let mut found = Vec::new();
        for id in ids {
            if let Some(id) = statement.query_row([id], |r| r.get(0)).optional()? {
                found.push(id);
            }
        }
        drop(statement);
        tx.commit()?;
        Ok(found)
    }
    pub(super) fn retention(&self, before: i64, limit: usize) -> Result<Vec<(String, String)>> {
        if !(1..=100).contains(&limit) {
            return Err(StoreError::TooLarge);
        }
        let _budget = ReadBudget::new(&self.conn);
        let mut statement=self.conn.prepare("SELECT id,created_at FROM sessions WHERE deleted=0 AND state='closed' AND recency<?1 ORDER BY recency DESC,id DESC LIMIT ?2")?;
        let rows = statement
            .query_map(rusqlite::params![before, limit as i64], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}
#[cfg(test)]
mod tests {
    use super::super::tests::{closed_document, document};
    use super::*;
    #[test]
    fn read_budget_interrupts_computation_instead_of_returning_an_empty_result() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let budget = ReadBudget::new(&db.conn);
        let result=db.conn.query_row("WITH RECURSIVE work(n) AS (VALUES(0) UNION ALL SELECT n+1 FROM work WHERE n<1000000000) SELECT sum(n) FROM work",[],|r|r.get::<_,i64>(0));
        assert!(matches!(
            result.map_err(StoreError::from),
            Err(StoreError::Expired)
        ));
        drop(budget);
        assert!(
            db.get("missing").unwrap().is_none(),
            "deadline hook is cleared after capture"
        );
    }
    #[test]
    fn cursor_binds_long_workspace_identity_without_exceeding_its_byte_cap() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(
            &dir.path().join("v1.sqlite"),
            &format!("/{}", "workspace".repeat(400)),
        )
        .unwrap();
        db.insert(closed_document("a")).unwrap();
        db.insert(closed_document("b")).unwrap();
        let first = db
            .history(HistoryQuery {
                limit: Some(1),
                ..Default::default()
            })
            .unwrap();
        let second = db
            .history(HistoryQuery {
                limit: Some(1),
                cursor: first.cursor,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(second.rows[0].id, "a");
    }
    #[test]
    fn all_unarchived_filter_excludes_archived_history() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        db.insert(closed_document("closed")).unwrap();
        db.insert(closed_document("archived")).unwrap();
        db.organize("archived", "2026-01-01T00:00:00Z", "0", "archived")
            .unwrap();
        for state in [None, Some("all".into())] {
            let page = db
                .history(HistoryQuery {
                    state,
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(
                page.rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
                ["closed"]
            );
        }
        let page = db
            .history(HistoryQuery {
                state: Some("archived".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.rows[0].id, "archived");
    }
    #[test]
    fn bounded_history_uses_indexed_summaries_ties_and_generation_bound_cursors() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        for id in ["a", "b", "c"] {
            db.insert(closed_document(id)).unwrap();
        }
        db.insert(document("live")).unwrap();
        let first = db
            .history(HistoryQuery {
                limit: Some(2),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            first.rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["c", "b"]
        );
        assert!(first.has_more);
        let second = db
            .history(HistoryQuery {
                limit: Some(2),
                cursor: first.cursor.clone(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            second
                .rows
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            ["a"]
        );
        assert!(!second.has_more);
        db.insert(closed_document("d")).unwrap();
        assert!(matches!(
            db.history(HistoryQuery {
                limit: Some(2),
                cursor: first.cursor,
                ..Default::default()
            }),
            Err(StoreError::Conflict)
        ));
        assert_eq!(db.live(&["live".into()]).unwrap().len(), 1);
        assert_eq!(db.exists(&["a".into(), "missing".into()]).unwrap(), ["a"]);
        let plan:String=db.conn.query_row("EXPLAIN QUERY PLAN SELECT id FROM sessions WHERE deleted=0 AND state='closed' ORDER BY recency DESC,id DESC LIMIT 100",[],|r|r.get(3)).unwrap();
        assert!(plan.contains("history_state"), "{plan}");
    }
    #[test]
    fn metadata_search_is_literal_bounded_and_transactionally_current() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut value = closed_document("search");
        value["label"] = serde_json::json!("SQLite durable history");
        db.insert(value).unwrap();
        let p = db
            .history(HistoryQuery {
                text: "sqlite dur".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(p.rows.len(), 1);
        for text in [
            "sqlite OR other",
            "sqlite*",
            "a b c d e f g h i",
            &"x".repeat(257),
        ] {
            assert!(
                db.history(HistoryQuery {
                    text: text.into(),
                    ..Default::default()
                })
                .is_err(),
                "{text}"
            );
        }
        assert!(db
            .history(HistoryQuery {
                text: "absent".into(),
                ..Default::default()
            })
            .unwrap()
            .rows
            .is_empty());
    }
    #[test]
    fn summary_budget_progresses_without_reading_or_truncating_canonical_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        for i in 0..40 {
            let mut value = closed_document(&format!("id-{i:02}"));
            value["label"] = serde_json::json!("é".repeat(20_000));
            value["summary"] = serde_json::json!("字".repeat(20_000));
            db.insert(value).unwrap();
        }
        let page = db
            .history(HistoryQuery {
                limit: Some(100),
                ..Default::default()
            })
            .unwrap();
        assert!(!page.rows.is_empty());
        assert!(serde_json::to_vec(&page).unwrap().len() <= 256 * 1024);
        for row in &page.rows {
            assert!(serde_json::to_vec(row).unwrap().len() <= 8192);
            assert!(row.preview_truncated);
        }
        assert_eq!(
            db.get("id-00").unwrap().unwrap()["label"]
                .as_str()
                .unwrap()
                .len(),
            40_000
        );
        assert!(db
            .history(HistoryQuery {
                limit: Some(101),
                ..Default::default()
            })
            .is_err());
    }
    #[test]
    fn retention_excludes_archived_unknown_and_tombstoned_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        for id in ["keep", "archived", "deleted"] {
            db.insert(closed_document(id)).unwrap();
        }
        db.insert(document("live")).unwrap();
        db.organize("archived", "2026-01-01T00:00:00Z", "0", "archived")
            .unwrap();
        db.begin_delete("deleted", "2026-01-01T00:00:00Z", "0", "0")
            .unwrap();
        let rows = db.retention(i64::MAX, 100).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "keep");
    }
}
