use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemory {
    pub id: String,
    pub agent_name: String,
    pub memory_type: String,
    pub flight_id: Option<String>,
    pub user_id: Option<String>,
    pub query: Option<String>,
    pub content: String,
    pub summary: Option<String>,
    pub tokens_used: i32,
    pub cost_usd: f64,
    pub model: Option<String>,
    pub embedding: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_accessed: Option<String>,
    pub access_count: i32,
    pub confidence_score: Option<f64>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub memory: AgentMemory,
    pub relevance_score: f64,
}

/// Store a new memory in the agent memory bank
pub fn store_memory(
    conn: &Connection,
    agent_name: &str,
    memory_type: &str,
    content: &str,
    query: Option<&str>,
    summary: Option<&str>,
    flight_id: Option<&str>,
    user_id: Option<&str>,
    model: Option<&str>,
    tokens_used: i32,
    cost_usd: f64,
    confidence_score: Option<f64>,
    ttl_hours: Option<i64>,
) -> Result<String> {
    let memory_id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let expires_at = ttl_hours.map(|hours| {
        (chrono::Utc::now() + chrono::Duration::hours(hours)).to_rfc3339()
    });

    conn.execute(
        "INSERT INTO agent_memory (
            id, agent_name, memory_type, flight_id, user_id,
            query, content, summary, tokens_used, cost_usd, model,
            created_at, expires_at, confidence_score
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            memory_id,
            agent_name,
            memory_type,
            flight_id,
            user_id,
            query,
            content,
            summary,
            tokens_used,
            cost_usd,
            model,
            created_at,
            expires_at,
            confidence_score,
        ],
    )?;

    Ok(memory_id)
}

/// Check if a similar memory exists (cache hit)
pub fn find_similar_memory(
    conn: &Connection,
    query: &str,
    memory_type: &str,
    max_age_hours: Option<i64>,
) -> Result<Option<AgentMemory>> {
    // Build SQL query with optional age filter
    let mut sql = String::from(
        "SELECT id, agent_name, memory_type, flight_id, user_id, query, content, summary,
         tokens_used, cost_usd, model, embedding, created_at, expires_at, last_accessed,
         access_count, confidence_score, verified
         FROM agent_memory
         WHERE memory_type = ?1
         AND (expires_at IS NULL OR expires_at > datetime('now'))"
    );

    if let Some(hours) = max_age_hours {
        sql.push_str(&format!(
            " AND created_at > datetime('now', '-{} hours')",
            hours
        ));
    }

    // Use full-text search for query matching
    sql.push_str(
        " AND id IN (
            SELECT rowid FROM agent_memory_fts WHERE agent_memory_fts MATCH ?2
        )"
    );

    sql.push_str(" ORDER BY created_at DESC LIMIT 1");

    let mut stmt = conn.prepare(&sql)?;

    let result = stmt.query_row(params![memory_type, query], |row| {
        Ok(AgentMemory {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            memory_type: row.get(2)?,
            flight_id: row.get(3)?,
            user_id: row.get(4)?,
            query: row.get(5)?,
            content: row.get(6)?,
            summary: row.get(7)?,
            tokens_used: row.get(8)?,
            cost_usd: row.get(9)?,
            model: row.get(10)?,
            embedding: row.get(11)?,
            created_at: row.get(12)?,
            expires_at: row.get(13)?,
            last_accessed: row.get(14)?,
            access_count: row.get(15)?,
            confidence_score: row.get(16)?,
            verified: row.get::<_, i32>(17)? == 1,
        })
    });

    match result {
        Ok(memory) => {
            // Update last_accessed and access_count
            update_memory_access(conn, &memory.id)?;
            Ok(Some(memory))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Update access tracking for a memory
fn update_memory_access(conn: &Connection, memory_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agent_memory
         SET last_accessed = datetime('now'),
             access_count = access_count + 1
         WHERE id = ?1",
        params![memory_id],
    )?;
    Ok(())
}

/// Search memories by full-text search
pub fn search_memories(
    conn: &Connection,
    search_query: &str,
    limit: usize,
) -> Result<Vec<MemorySearchResult>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.agent_name, m.memory_type, m.flight_id, m.user_id, m.query, m.content,
         m.summary, m.tokens_used, m.cost_usd, m.model, m.embedding, m.created_at, m.expires_at,
         m.last_accessed, m.access_count, m.confidence_score, m.verified, fts.rank
         FROM agent_memory m
         JOIN agent_memory_fts fts ON m.rowid = fts.rowid
         WHERE fts.agent_memory_fts MATCH ?1
         AND (m.expires_at IS NULL OR m.expires_at > datetime('now'))
         ORDER BY fts.rank DESC, m.access_count DESC
         LIMIT ?2"
    )?;

    let results = stmt
        .query_map(params![search_query, limit], |row| {
            Ok(MemorySearchResult {
                memory: AgentMemory {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    memory_type: row.get(2)?,
                    flight_id: row.get(3)?,
                    user_id: row.get(4)?,
                    query: row.get(5)?,
                    content: row.get(6)?,
                    summary: row.get(7)?,
                    tokens_used: row.get(8)?,
                    cost_usd: row.get(9)?,
                    model: row.get(10)?,
                    embedding: row.get(11)?,
                    created_at: row.get(12)?,
                    expires_at: row.get(13)?,
                    last_accessed: row.get(14)?,
                    access_count: row.get(15)?,
                    confidence_score: row.get(16)?,
                    verified: row.get::<_, i32>(17)? == 1,
                },
                relevance_score: row.get::<_, f64>(18)?.abs(), // FTS5 rank is negative
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Get all memories for a specific flight
pub fn get_flight_memories(
    conn: &Connection,
    flight_id: &str,
) -> Result<Vec<AgentMemory>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, memory_type, flight_id, user_id, query, content, summary,
         tokens_used, cost_usd, model, embedding, created_at, expires_at, last_accessed,
         access_count, confidence_score, verified
         FROM agent_memory
         WHERE flight_id = ?1
         AND (expires_at IS NULL OR expires_at > datetime('now'))
         ORDER BY created_at DESC"
    )?;

    let memories = stmt
        .query_map(params![flight_id], |row| {
            Ok(AgentMemory {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                memory_type: row.get(2)?,
                flight_id: row.get(3)?,
                user_id: row.get(4)?,
                query: row.get(5)?,
                content: row.get(6)?,
                summary: row.get(7)?,
                tokens_used: row.get(8)?,
                cost_usd: row.get(9)?,
                model: row.get(10)?,
                embedding: row.get(11)?,
                created_at: row.get(12)?,
                expires_at: row.get(13)?,
                last_accessed: row.get(14)?,
                access_count: row.get(15)?,
                confidence_score: row.get(16)?,
                verified: row.get::<_, i32>(17)? == 1,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(memories)
}

/// Get recent memories (for Memory Bank view)
pub fn get_recent_memories(
    conn: &Connection,
    limit: usize,
    agent_filter: Option<&str>,
) -> Result<Vec<AgentMemory>> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(agent) = agent_filter {
        (
            "SELECT id, agent_name, memory_type, flight_id, user_id, query, content, summary,
             tokens_used, cost_usd, model, embedding, created_at, expires_at, last_accessed,
             access_count, confidence_score, verified
             FROM agent_memory
             WHERE agent_name = ?1
             AND (expires_at IS NULL OR expires_at > datetime('now'))
             ORDER BY created_at DESC
             LIMIT ?2".to_string(),
            vec![Box::new(agent.to_string()), Box::new(limit)]
        )
    } else {
        (
            "SELECT id, agent_name, memory_type, flight_id, user_id, query, content, summary,
             tokens_used, cost_usd, model, embedding, created_at, expires_at, last_accessed,
             access_count, confidence_score, verified
             FROM agent_memory
             WHERE (expires_at IS NULL OR expires_at > datetime('now'))
             ORDER BY created_at DESC
             LIMIT ?1".to_string(),
            vec![Box::new(limit)]
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let memories: Vec<AgentMemory> = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(AgentMemory {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                memory_type: row.get(2)?,
                flight_id: row.get(3)?,
                user_id: row.get(4)?,
                query: row.get(5)?,
                content: row.get(6)?,
                summary: row.get(7)?,
                tokens_used: row.get(8)?,
                cost_usd: row.get(9)?,
                model: row.get(10)?,
                embedding: row.get(11)?,
                created_at: row.get(12)?,
                expires_at: row.get(13)?,
                last_accessed: row.get(14)?,
                access_count: row.get(15)?,
                confidence_score: row.get(16)?,
                verified: row.get::<_, i32>(17)? == 1,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(memories)
}

/// Delete expired memories
pub fn cleanup_expired_memories(conn: &Connection) -> Result<usize> {
    let deleted = conn.execute(
        "DELETE FROM agent_memory WHERE expires_at IS NOT NULL AND expires_at < datetime('now')",
        [],
    )?;
    Ok(deleted)
}

/// Get memory statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub memories_by_agent: Vec<(String, usize)>,
    pub memories_by_type: Vec<(String, usize)>,
}

pub fn get_memory_stats(conn: &Connection) -> Result<MemoryStats> {
    // Total memories
    let total_memories: usize = conn.query_row(
        "SELECT COUNT(*) FROM agent_memory WHERE (expires_at IS NULL OR expires_at > datetime('now'))",
        [],
        |row| row.get(0),
    )?;

    // Total tokens and cost
    let (total_tokens, total_cost): (i64, f64) = conn.query_row(
        "SELECT COALESCE(SUM(tokens_used), 0), COALESCE(SUM(cost_usd), 0.0)
         FROM agent_memory
         WHERE (expires_at IS NULL OR expires_at > datetime('now'))",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Memories by agent
    let mut stmt = conn.prepare(
        "SELECT agent_name, COUNT(*) FROM agent_memory
         WHERE (expires_at IS NULL OR expires_at > datetime('now'))
         GROUP BY agent_name"
    )?;
    let memories_by_agent: Vec<(String, usize)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Memories by type
    let mut stmt = conn.prepare(
        "SELECT memory_type, COUNT(*) FROM agent_memory
         WHERE (expires_at IS NULL OR expires_at > datetime('now'))
         GROUP BY memory_type"
    )?;
    let memories_by_type: Vec<(String, usize)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(MemoryStats {
        total_memories,
        total_tokens,
        total_cost,
        memories_by_agent,
        memories_by_type,
    })
}
