use crate::protos::agent::SysInfo;

use actix_http::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Serialize, Deserialize)]
pub struct AgentRequest {
    pub guid: String,
}

#[derive(Serialize, Deserialize)]
pub struct AgentCreateRequest {
    pub description: String,
    pub agent_type: String,
    pub endpoint: String,
}

impl Responder for AgentCreateRequest {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct Agent {
    pub guid: String,
    pub description: String,
    pub agent_type: String,
    pub endpoint: String,
    pub status: String,
    pub free_cpus: Option<i64>,
    pub free_ram: Option<i64>,
    pub cpus: Option<i64>,
    pub ram: Option<i64>,
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
            SELECT guid, description, agent_type, endpoint, status, free_cpus, free_ram, cpus, ram
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
            free_cpus: rec.free_cpus,
            free_ram: rec.free_ram,
            cpus: rec.cpus,
            ram: rec.ram,
        })
        .collect();

        Ok(agents)
    }

    pub async fn get_by_guid(guid: &String, pool: &SqlitePool) -> Result<Option<Agent>> {
        let rec = sqlx::query!(
            r#"
            SELECT guid, description, agent_type, endpoint, status, free_cpus, free_ram, cpus, ram
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
            free_cpus: rec.free_cpus,
            free_ram: rec.free_ram,
            cpus: rec.cpus,
            ram: rec.ram,
        }))
    }

    pub async fn create(agent: Agent, pool: &SqlitePool) -> Result<Agent> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO agents (guid, description, agent_type, endpoint, status, cpus, ram)
            VALUES($1, $2, $3, $4, $5, NULL, NULL)
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

    pub async fn update_sys_info(
        guid: &String,
        sys_info: &SysInfo,
        pool: &SqlitePool,
    ) -> Result<bool> {
        let cpus = i64::try_from(sys_info.cpus).unwrap_or(0);
        let ram = i64::try_from(sys_info.ram).unwrap_or(0);
        let rows_affected = sqlx::query!(
            r#"
            UPDATE agents
            SET free_cpus = $2, free_ram = $3, cpus = $4, ram = $5
            WHERE guid = $1
            "#,
            guid,
            cpus,
            ram,
            cpus,
            ram
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn update_status(guid: &String, status: &str, pool: &SqlitePool) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE agents
            SET status = $2
            WHERE guid = $1
            "#,
            guid,
            status
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }
}
