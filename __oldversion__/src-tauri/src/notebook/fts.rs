use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::RepoError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryHit {
    pub entity_type: String,
    pub entity_id: String,
    pub sort_ms: i64,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CursorPayload {
    sort_ms: i64,
    entity_id: String,
}

pub fn search(
    connection: &Connection,
    query: &str,
    cursor: Option<&str>,
    page_size: usize,
    cursor_secret: &[u8; 32],
) -> Result<Page<HistoryHit>, RepoError> {
    if page_size == 0 || page_size > 100 {
        return Err(RepoError::InvalidRequest);
    }
    let cursor = cursor
        .map(|value| decode_cursor(value, cursor_secret))
        .transpose()?;
    let match_expression = match_expression(query)?;
    let (cursor_sort, cursor_id) = cursor
        .map(|payload| (Some(payload.sort_ms), Some(payload.entity_id)))
        .unwrap_or((None, None));
    let limit = i64::try_from(page_size + 1).map_err(|_| RepoError::InvalidRequest)?;

    let mut statement = connection
        .prepare(
            "SELECT entity_type, entity_id, CAST(sort_ms AS INTEGER), content
             FROM history_fts
             WHERE history_fts MATCH ?1
               AND (
                 ?2 IS NULL
                 OR CAST(sort_ms AS INTEGER) < ?2
                 OR (CAST(sort_ms AS INTEGER) = ?2 AND entity_id < ?3)
               )
             ORDER BY CAST(sort_ms AS INTEGER) DESC, entity_id DESC
             LIMIT ?4",
        )
        .map_err(|_| RepoError::NotebookInvalid)?;
    let rows = statement
        .query_map(
            params![match_expression, cursor_sort, cursor_id, limit],
            |row| {
                Ok(HistoryHit {
                    entity_type: row.get(0)?,
                    entity_id: row.get(1)?,
                    sort_ms: row.get(2)?,
                    content: row.get(3)?,
                })
            },
        )
        .map_err(|_| RepoError::InvalidRequest)?;
    let mut items = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| RepoError::NotebookInvalid)?;
    let has_more = items.len() > page_size;
    items.truncate(page_size);
    let next_cursor = if has_more {
        items
            .last()
            .map(|hit| {
                encode_cursor(
                    &CursorPayload {
                        sort_ms: hit.sort_ms,
                        entity_id: hit.entity_id.clone(),
                    },
                    cursor_secret,
                )
            })
            .transpose()?
    } else {
        None
    };
    Ok(Page { items, next_cursor })
}

fn match_expression(query: &str) -> Result<String, RepoError> {
    let tokens = query.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err(RepoError::InvalidRequest);
    }
    Ok(tokens
        .into_iter()
        .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND "))
}

fn encode_cursor(payload: &CursorPayload, secret: &[u8; 32]) -> Result<String, RepoError> {
    let payload = serde_json::to_vec(payload).map_err(|_| RepoError::InvalidCursor)?;
    let signature = cursor_signature(secret, &payload);
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload),
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

fn decode_cursor(value: &str, secret: &[u8; 32]) -> Result<CursorPayload, RepoError> {
    let (payload, signature) = value.split_once('.').ok_or(RepoError::InvalidCursor)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| RepoError::InvalidCursor)?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| RepoError::InvalidCursor)?;
    if signature.as_slice() != cursor_signature(secret, &payload) {
        return Err(RepoError::InvalidCursor);
    }
    serde_json::from_slice(&payload).map_err(|_| RepoError::InvalidCursor)
}

fn cursor_signature(secret: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"mtgo-notes-history-cursor-v1");
    digest.update(secret);
    digest.update(payload);
    digest.finalize().into()
}
