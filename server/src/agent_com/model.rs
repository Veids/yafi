use chrono;

use actix_http::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::agent_com::routes::JobInfo;
use crate::protos::agent::{JobCreateRequest, JobInfoContainerList, SysInfo};

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

#[derive(Debug)]
pub struct JobRequest {
    pub agent_guid: String,
    pub request: JobCreateRequest,
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

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct Job {
    pub agent_guid: String,
    pub collection_guid: String,
    pub master: bool,
    pub cpus: u64,
    pub ram: u64,
    pub last_msg: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct JobInfoResponse {
    pub job_collection: JobCollection,
    pub jobs: Vec<Job>,
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

    pub async fn schedule_job(job_info: &JobInfo, pool: &SqlitePool) -> Result<Vec<JobRequest>> {
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
        let mut scheduled_jobs: Vec<JobRequest> = Vec::new();
        let mut master = true;
        for agent in rec.iter() {
            scheduled_jobs.push(JobRequest {
                agent_guid: agent.guid.clone(),
                request: JobCreateRequest {
                    job_guid: job_info.guid.clone(),
                    image: job_info.image.clone(),
                    master: master,
                    cpus: std::cmp::min(rest_cpus, agent.free_cpus.unwrap_or(0) as u64),
                    ram: std::cmp::min(rest_ram, agent.free_ram.unwrap_or(0) as u64),
                    timeout: job_info.timeout.clone(),
                    target: job_info.target.clone(),
                    corpus: job_info.corpus.clone(),
                    last_msg: "".to_string(),
                    status: "init".to_string(),
                },
            });
            rest_cpus -= std::cmp::min(rest_cpus, agent.free_cpus.unwrap_or(0) as u64);
            rest_ram -= std::cmp::min(rest_ram, agent.free_ram.unwrap_or(0) as u64);
            master = false;
        }

        for job in scheduled_jobs.iter() {
            let cpus = i64::try_from(job.request.cpus)?;
            let ram = i64::try_from(job.request.ram)?;
            sqlx::query!(
                r#"
                UPDATE agents
                SET free_cpus = free_cpus - $2, free_ram = free_ram - $3
                WHERE guid = $1
                "#,
                job.agent_guid,
                cpus,
                ram
            )
            .execute(&mut tx)
            .await?;

            let master = i64::try_from(job.request.master)?;
            sqlx::query!(
                r#"
                INSERT INTO jobs (agent_guid, collection_guid, master, cpus, ram, last_msg, status)
                VALUES($1, $2, $3, $4, $5, $6, $7)
                "#,
                job.agent_guid,
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
        Ok(scheduled_jobs)
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
                "completed" => job_stats.completed = stat.count.unwrap() as u64,
                "error" => job_stats.error = stat.count.unwrap() as u64,
                _ => {}
            }
        }

        Ok(job_stats)
    }

    pub async fn get_job(guid: &String, pool: &SqlitePool) -> Result<JobInfoResponse> {
        let rec = sqlx::query!(
            "
            SELECT guid, name, description, creation_date, agent_type, cpus, ram, timeout, target, corpus, status
            FROM job_collection
            WHERE guid = $1
            ",
            guid
        )
        .fetch_one(pool)
        .await?;

        let job_collection = JobCollection {
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
            status: rec.status,
        };

        let jobs = sqlx::query!(
            "
            SELECT agent_guid, collection_guid, master, cpus, ram, last_msg, status
            FROM jobs
            WHERE collection_guid = $1
            ",
            guid
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|rec| Job {
            agent_guid: rec.agent_guid,
            collection_guid: rec.collection_guid,
            master: rec.master,
            cpus: rec.cpus.unwrap() as u64,
            ram: rec.ram.unwrap() as u64,
            last_msg: rec.last_msg,
            status: rec.status,
        })
        .collect();

        Ok(JobInfoResponse {
            job_collection: job_collection,
            jobs: jobs,
        })
    }

    pub async fn set_job_status(
        agent_guid: &String,
        job_guid: &String,
        last_msg: &str,
        pool: &SqlitePool,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE jobs
            SET status = $3
            WHERE collection_guid = $1 AND agent_guid = $2
            "#,
            job_guid,
            agent_guid,
            last_msg
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_job_last_msg(
        agent_guid: &String,
        job_guid: &String,
        status: &str,
        pool: &SqlitePool,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE jobs
            SET last_msg = $3
            WHERE collection_guid = $1 AND agent_guid = $2
            "#,
            job_guid,
            agent_guid,
            status
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn complete_job(
        agent_guid: &String,
        job_guid: &String,
        last_msg: &String,
        status: &str,
        pool: &SqlitePool,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;

        let rec = sqlx::query!(
            r#"
            SELECT id, agent_guid, cpus, ram
            FROM jobs
            WHERE collection_guid = $1 AND agent_guid = $2 AND freed != 1
            "#,
            job_guid,
            agent_guid
        )
        .fetch_all(&mut tx)
        .await?;

        if rec.len() == 0 {
            return Ok(());
        }

        for job in rec.iter() {
            sqlx::query!(
                r#"
                UPDATE jobs
                SET freed = 1, last_msg = $2, status = $3
                WHERE id = $1
                "#,
                job.id,
                last_msg,
                status
            )
            .execute(&mut tx)
            .await?;

            sqlx::query!(
                r#"
                UPDATE agents
                SET free_cpus = free_cpus + $1, free_ram = free_ram + $2
                WHERE guid = $3
                "#,
                job.cpus,
                job.ram,
                job.agent_guid
            )
            .execute(&mut tx)
            .await?;
            break;
        }

        //Propagate status
        let rec = sqlx::query!(
            r#"
            SELECT id FROM jobs
            WHERE collection_guid = $1 AND freed != 1
            LIMIT 1
            "#,
            job_guid
        )
        .fetch_one(&mut tx)
        .await;

        //Status from error cannot be changed to anything
        if rec.is_err() {
            sqlx::query!(
                r#"
                UPDATE job_collection
                SET status = $2
                WHERE guid = $1 AND status != 'error'
                "#,
                job_guid,
                status
            )
            .execute(&mut tx)
            .await?;
        }

        tx.commit().await.unwrap();
        Ok(())
    }

    pub async fn sync_jobs(
        agent_guid: &String,
        jobs: JobInfoContainerList,
        pool: &SqlitePool,
    ) -> Result<()> {
        // let mut tx = pool.begin().await?;

        let collection_guids = jobs
            .jobs
            .iter()
            .map(|job| job.job_guid.clone())
            .collect::<Vec<String>>()
            .join(",");

        let to_complete = sqlx::query!(
            r#"
            SELECT collection_guid
            FROM jobs
            WHERE status IN ("init", "alive") AND agent_guid == $1 AND collection_guid NOT IN ($2)
            "#,
            agent_guid,
            collection_guids
        )
        .fetch_all(pool)
        .await?;

        for collection in to_complete.iter() {
            Self::complete_job(
                &agent_guid,
                &collection.collection_guid,
                &"unknown".to_string(),
                "completed",
                &pool,
            )
            .await?;
        }

        for job in jobs.jobs.iter() {
            if job.status == "error" || job.status == "completed" {
                Self::complete_job(
                    &agent_guid,
                    &job.job_guid,
                    &job.last_msg,
                    job.status.as_ref(),
                    &pool,
                )
                .await?;
            } else {
                sqlx::query!(
                    r#"
                    UPDATE jobs
                    SET status = $3, last_msg = $4
                    WHERE agent_guid = $1 AND collection_guid = $2
                    "#,
                    agent_guid,
                    job.job_guid,
                    job.status,
                    job.last_msg
                )
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    // pub async fn destroy_job(guid: &String, pool: &SqlitePool) -> Result<()> {
    //     let mut tx = pool.begin().await?;

    //     let rec = sqlx::query!(
    //         r#"
    //         SELECT agent_guid, SUM(cpus) as cpus, SUM(ram) as ram
    //         FROM jobs
    //         WHERE collection_guid = $1
    //         GROUP BY agent_guid
    //         "#,
    //         guid
    //     ).execute(&mut tx)
    //     .await?;

    //     for job in rec.iter() {
    //         sqlx::query!(
    //         r#"
    //         UPDATE agents
    //         SET free_cpus = free_cpus + $1, free_ram = free_ram + $2
    //         WHERE
    //         "#
    //         );
    //     }

    //     sqlx::query!(
    //         r#"
    //         DELETE FROM agents
    //         WHERE guid = $1
    //         "#,
    //         guid
    //     )
    //     .execute(&mut tx)
    //     .await?;

    //     tx.commit().await.unwrap();
    //     Ok(())
    // }
}
