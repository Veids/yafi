use chrono;

use actix_http::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::protos::agent::{JobInfo, SysInfo};

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

#[derive(Serialize, Deserialize)]
pub struct JobCreateRequest {
    pub name: String,
    pub description: String,
    pub agent_type: String,
    pub docker_image: String,
    pub cpus: u64,
    pub ram: u64,
}

impl Responder for AgentCreateRequest {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

#[derive(Debug)]
pub struct ScheduledAgent {
    pub guid: String,
    pub cpus: u64,
    pub ram: u64,
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

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct JobCollection {
    pub guid: String,
    pub name: String,
    pub description: String,
    pub agent_type: String,
    pub creation_date: String,
    pub cpus: u64,
    pub ram: u64,
    pub timeout: String,
    pub target: String,
    pub corpus: String,
    pub status: String,
}

impl Responder for JobCollection {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct JobStats {
    pub alive: u64,
    pub completed: u64,
    pub error: u64,
}

impl Responder for JobStats {
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

    pub async fn get_all_collections(pool: &SqlitePool) -> Result<Vec<JobCollection>> {
        let job_collection = sqlx::query!(
            r#"
              SELECT guid, name, description, creation_date, agent_type, cpus, ram, timeout, target, corpus, status
              FROM job_collection
            "#
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|rec| JobCollection {
            guid: rec.guid,
            name: rec.name,
            description: rec.description,
            creation_date: rec.creation_date,
            agent_type: rec.agent_type,
            cpus: rec.cpus.unwrap() as u64,
            ram: rec.ram.unwrap() as u64,
            timeout: rec.timeout,
            target: rec.target,
            corpus: rec.corpus,
            status: rec.status
        })
        .collect();

        Ok(job_collection)
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
        let mut tx = pool.begin().await?;

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
        .execute(&mut tx)
        .await?
        .rows_affected();

        tx.commit().await.unwrap();

        Ok(rows_affected > 0)
    }

    pub async fn update_status(guid: &String, status: &str, pool: &SqlitePool) -> Result<bool> {
        let mut tx = pool.begin().await?;

        let rows_affected = sqlx::query!(
            r#"
            UPDATE agents
            SET status = $2
            WHERE guid = $1
            "#,
            guid,
            status
        )
        .execute(&mut tx)
        .await?
        .rows_affected();

        tx.commit().await.unwrap();

        Ok(rows_affected > 0)
    }

    pub async fn schedule_job(
        job_info: &JobInfo,
        pool: &SqlitePool,
    ) -> Result<Vec<ScheduledAgent>> {
        let mut tx = pool.begin().await?;

        let rec = sqlx::query!(
            r#"
            SELECT guid, free_cpus, free_ram
            FROM agents
            WHERE status == 'up' AND agent_type = $1 AND free_cpus > 0 AND free_ram > 0
            ORDER BY free_cpus DESC
            "#,
            job_info.agent_type,
        )
        .fetch_all(&mut tx)
        .await?;

        let total_free_cpus: u64 = rec
            .iter()
            .map(|rec| rec.free_cpus.unwrap_or(0) as u64)
            .sum();
        if total_free_cpus < job_info.cpus {
            return Err(anyhow::anyhow!("Insuficient amount of free cpus"));
        }

        let mut rest_cpus = job_info.cpus;
        let mut rest_ram = job_info.ram;
        let mut scheduled_agents: Vec<ScheduledAgent> = Vec::new();
        for agent in rec.iter() {
            scheduled_agents.push(ScheduledAgent {
                guid: agent.guid.clone(),
                cpus: std::cmp::min(rest_cpus, agent.free_cpus.unwrap_or(0) as u64),
                ram: std::cmp::min(rest_ram, agent.free_ram.unwrap_or(0) as u64),
            });
            rest_cpus -= std::cmp::min(rest_cpus, agent.free_cpus.unwrap_or(0) as u64);
            rest_ram -= std::cmp::min(rest_ram, agent.free_ram.unwrap_or(0) as u64);
        }

        for (i, agent) in scheduled_agents.iter().enumerate() {
            let cpus = i64::try_from(agent.cpus)?;
            let ram = i64::try_from(agent.ram)?;
            sqlx::query!(
                r#"
                UPDATE agents
                SET free_cpus = free_cpus - $2, free_ram = free_ram - $3
                WHERE guid = $1
                "#,
                agent.guid,
                cpus,
                ram
            )
            .execute(&mut tx)
            .await?;

            let master = if i == 0 { 1 } else { 0 };
            sqlx::query!(
                r#"
                INSERT INTO jobs (agent_guid, collection_guid, master, cpus, ram, last_msg, status)
                VALUES($1, $2, $3, $4, $5, $6, $7)
                "#,
                agent.guid,
                job_info.guid,
                master,
                cpus,
                ram,
                "",
                "init"
            )
            .execute(&mut tx)
            .await?;
        }

        let cpus = i64::try_from(job_info.cpus)?;
        let ram = i64::try_from(job_info.ram)?;
        let now = chrono::offset::Utc::now().to_string();
        sqlx::query!(
            r#"
            INSERT INTO job_collection (guid, name, description, creation_date, agent_type, cpus, ram, timeout, target, corpus, status)
            VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            job_info.guid,
            job_info.name,
            job_info.description,
            now,
            job_info.agent_type,
            cpus,
            ram,
            job_info.timeout,
            job_info.target,
            job_info.corpus,
            "init"
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await.unwrap();
        Ok(scheduled_agents)
    }

    pub async fn get_job_stats(pool: &SqlitePool) -> Result<JobStats> {
        let rec = sqlx::query!(
            "
            SELECT status, COUNT() as count
            FROM job_collection 
            GROUP BY status
            "
        )
        .fetch_all(pool)
        .await?;

        if rec.len() == 0 {
            return Ok(JobStats {
                ..Default::default()
            });
        }

        let mut job_stats = JobStats {
            ..Default::default()
        };

        for stat in rec.iter() {
            match stat.status.as_ref() {
                "up" => job_stats.alive += stat.count.unwrap() as u64,
                "init" => job_stats.alive += stat.count.unwrap() as u64,
                "completed" => job_stats.alive = stat.count.unwrap() as u64,
                "error" => job_stats.alive = stat.count.unwrap() as u64,
                _ => {}
            }
        }

        Ok(job_stats)
    }
}
