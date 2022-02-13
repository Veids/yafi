use crate::handlers::agent::JobInfo;
use crate::protos::agent::{JobCreateRequest, JobInfoContainerList};

use actix_http::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};

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

#[derive(Debug)]
pub struct JobRequest {
    pub agent_guid: String,
    pub request: JobCreateRequest,
}

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct Job {
    pub agent_guid: String,
    pub collection_guid: String,
    pub idx: u64,
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

impl Job {
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
        for (i, agent) in rec.iter().enumerate() {
            scheduled_jobs.push(JobRequest {
                agent_guid: agent.guid.clone(),
                request: JobCreateRequest {
                    job_guid: job_info.guid.clone(),
                    image: job_info.image.clone(),
                    idx: i as u64,
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
        }

        for job in scheduled_jobs.iter() {
            let cpus = i64::try_from(job.request.cpus)?;
            let ram = i64::try_from(job.request.ram)?;
            let idx = i64::try_from(job.request.idx)?;
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

            sqlx::query!(
                r#"
                INSERT INTO jobs (agent_guid, collection_guid, idx, cpus, ram, last_msg, status, freed)
                VALUES($1, $2, $3, $4, $5, $6, $7, 0)
                "#,
                job.agent_guid,
                job_info.guid,
                idx,
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
            r#"
            SELECT status, COUNT(*) as count
            FROM job_collection 
            GROUP BY status
            "#
        )
        .fetch_all(pool)
        .await?;

        if rec.is_empty() {
            return Ok(JobStats {
                ..Default::default()
            });
        }

        let mut job_stats = JobStats {
            ..Default::default()
        };

        for stat in rec.iter() {
            match stat.status.as_ref() {
                "alive" => job_stats.alive += stat.count.unwrap() as u64,
                "init" => job_stats.alive += stat.count.unwrap() as u64,
                "completed" => job_stats.completed = stat.count.unwrap() as u64,
                "error" => job_stats.error = stat.count.unwrap() as u64,
                _ => {}
            }
        }

        Ok(job_stats)
    }

    pub async fn get_job(guid: &str, pool: &SqlitePool) -> Result<JobInfoResponse> {
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
            SELECT agent_guid, collection_guid, idx, cpus, ram, last_msg, status
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
            idx: rec.idx.unwrap() as u64,
            cpus: rec.cpus.unwrap() as u64,
            ram: rec.ram.unwrap() as u64,
            last_msg: rec.last_msg,
            status: rec.status,
        })
        .collect();

        Ok(JobInfoResponse {
            job_collection,
            jobs,
        })
    }

    async fn propagate_status(job_guid: &str, pool: &SqlitePool) -> Result<()> {
        let mut tx = pool.begin().await?;

        let statuses = sqlx::query!(
            r#"
            SELECT status, COUNT(*) as count
            FROM jobs
            WHERE collection_guid = $1
            "#,
            job_guid
        )
        .fetch_all(&mut tx)
        .await?;

        let mut errors = 0;
        let mut alive = 0;
        // let mut completed = 0;
        let mut init = 0;

        for stat in statuses.iter() {
            match stat.status.as_ref() {
                "alive" => alive = stat.count.unwrap() as u64,
                "init" => init = stat.count.unwrap() as u64 as u64,
                // "completed" => completed = stat.count.unwrap() as u64,
                "error" => errors = stat.count.unwrap() as u64,
                _ => {}
            }
        }

        let status;
        if errors != 0 {
            status = "error";
        } else if init != 0 {
            status = "init";
        } else if alive != 0 {
            status = "alive";
        } else {
            status = "completed";
        }

        sqlx::query!(
            r#"
            UPDATE job_collection
            SET status = $2
            WHERE guid = $1
            "#,
            job_guid,
            status
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await.unwrap();
        Ok(())
    }

    pub async fn set_job_status(
        agent_guid: &str,
        job_guid: &str,
        status: &str,
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
            status
        )
        .execute(pool)
        .await?;

        Self::propagate_status(job_guid, pool).await?;

        Ok(())
    }

    pub async fn set_job_last_msg(
        agent_guid: &str,
        job_guid: &str,
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
        agent_guid: &str,
        job_guid: &str,
        last_msg: &str,
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

        if rec.is_empty() {
            return Ok(());
        }

        if let Some(job) = rec.get(0) {
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
        }

        tx.commit().await.unwrap();

        Self::propagate_status(job_guid, pool).await?;

        ////Propagate status
        //let rec = sqlx::query!(
        //    r#"
        //    SELECT id FROM jobs
        //    WHERE collection_guid = $1 AND freed != 1
        //    LIMIT 1
        //    "#,
        //    job_guid
        //)
        //.fetch_one(&mut tx)
        //.await;

        ////Status from error cannot be changed to anything
        //if rec.is_err() {
        //    sqlx::query!(
        //        r#"
        //        UPDATE job_collection
        //        SET status = $2
        //        WHERE guid = $1 AND status != 'error'
        //        "#,
        //        job_guid,
        //        status
        //    )
        //    .execute(&mut tx)
        //    .await?;
        //}

        //tx.commit().await.unwrap();
        Ok(())
    }

    pub async fn sync_jobs(
        agent_guid: &str,
        jobs: JobInfoContainerList,
        pool: &SqlitePool,
    ) -> Result<()> {
        let collection_guids = jobs
            .jobs
            .iter()
            .map(|job| job.job_guid.as_ref())
            .collect::<Vec<&str>>();

        let query = format!(
            r#"
            SELECT collection_guid
            FROM jobs
            WHERE status IN ("init", "alive") AND agent_guid == ? AND collection_guid NOT IN ({})
            "#,
            (0..collection_guids.len())
                .map(|_| "?")
                .collect::<Vec<&str>>()
                .join(",")
        );

        let mut q = sqlx::query(&query);
        q = q.bind(agent_guid);

        for x in &collection_guids {
            q = q.bind(x);
        }

        let to_complete = q.fetch_all(pool).await?;

        for collection in to_complete.iter() {
            let id: String = collection.try_get(0)?;
            info!("Completing {}", id);
            Self::complete_job(
                agent_guid,
                collection.try_get(0)?,
                &"unknown".to_string(),
                "completed",
                pool,
            )
            .await?;
        }

        for job in jobs.jobs.iter() {
            if job.status == "error" || job.status == "completed" {
                Self::complete_job(
                    agent_guid,
                    &job.job_guid,
                    &job.last_msg,
                    job.status.as_ref(),
                    pool,
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
}
