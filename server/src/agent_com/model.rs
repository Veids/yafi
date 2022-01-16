use actix_http::body::BoxBody;
use actix_web::{Error, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, SqlitePool};
use tokio::sync::mpsc::Sender;

use crate::agent_processor::AgentUpdate;

#[derive(Serialize, Deserialize)]
pub struct AgentRequest {
    pub guid: String,
}

#[derive(Serialize, Deserialize)]
pub struct AgentCreateRequest {
    pub description: String,
    pub agent_type: String,
    pub endpoint: String
}

impl Responder for AgentCreateRequest {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub guid: String,
    pub description: String,
    pub agent_type: String,
    pub endpoint: String,
    pub status: String,
}

impl Responder for Agent {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

impl Agent {
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Agent>> {
        let agents = sqlx::query!(
            r#"
            SELECT guid, description, agent_type, endpoint, status
            FROM agents
            ORDER BY guid
            "#
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|rec| Agent {
            guid: rec.guid,
            description: rec.description,
            agent_type: rec.agent_type,
            endpoint: rec.endpoint,
            status: rec.status,
        })
        .collect();

        Ok(agents)
    }

    pub async fn get_by_guid(guid: &String, pool: &SqlitePool) -> Result<Option<Agent>> {
        let rec = sqlx::query!(
            r#"
            SELECT guid, description, agent_type, endpoint, status
            FROM agents
            WHERE guid = $1
            "#,
            guid
        )
        .fetch_optional(&*pool)
        .await?;

        Ok(rec.map(|rec| Agent {
            guid: rec.guid,
            description: rec.description,
            agent_type: rec.agent_type,
            endpoint: rec.endpoint,
            status: rec.status,
        }))
    }

    pub async fn create(
        agent: Agent,
        pool: &SqlitePool
    ) -> Result<Agent> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO agents (guid, description, agent_type, endpoint, status)
            VALUES($1, $2, $3, $4, $5)
            "#,
            agent.guid,
            agent.description,
            agent.agent_type,
            agent.endpoint,
            agent.status
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await.unwrap();

        Ok(agent)
    }

    pub async fn delete(guid: String, pool: &SqlitePool) -> Result<String> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            r#"
            DELETE FROM agents
            WHERE guid = $1
            "#,
            guid
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await.unwrap();
        Ok(guid)
    }
}
