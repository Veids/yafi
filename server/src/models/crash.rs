use std::{fs, io};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::protos::agent::CrashMsg;
use crate::utils::get_job_dir;

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct CrashStats {
    pub total: u64,
}

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct Crash {
    pub guid: String,
    pub name: String,
    pub collection_guid: String,
    pub analyzed: Option<String>,
    pub hash: String,
    pub creation_date: String,
    pub size: i64,
}

impl Crash {
    pub async fn new_crash(crash: &CrashMsg, pool: &SqlitePool) -> Result<()> {
        let guid = Uuid::new_v4().to_string();
        let crash_path = get_job_dir(&crash.job_guid)
            .join("crashes")
            .join(&crash.name);

        let mut file = fs::File::open(&crash_path)?;
        let mut hasher = Sha3_256::new();
        io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        let hash_str = format!("{:x}", hash);

        let now = chrono::offset::Utc::now().to_string();
        let metadata = fs::metadata(&crash_path)?;
        let size = i64::try_from(metadata.len())?;

        sqlx::query!(
            r#"
            INSERT INTO crashes (guid, name, collection_guid, analyzed, hash, creation_date, size)
            VALUES($1, $2, $3, NULL, $4, $5, $6)
            "#,
            guid,
            crash.name,
            crash.job_guid,
            hash_str,
            now,
            size
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_crashes(pool: &SqlitePool) -> Result<Vec<Crash>> {
        Ok(sqlx::query_as!(
            Crash,
            r#"
            SELECT guid, name, collection_guid, analyzed, hash, creation_date, size
            FROM crashes
            "#
        )
        .fetch_all(pool)
        .await?)
    }

    pub async fn get_all_crashes_by_job(job_guid: &str, pool: &SqlitePool) -> Result<Vec<Crash>> {
        Ok(sqlx::query_as!(
            Crash,
            r#"
            SELECT guid, name, collection_guid, analyzed, hash, creation_date, size
            FROM crashes
            WHERE collection_guid = $1
            "#,
            job_guid
        )
        .fetch_all(pool)
        .await?)
    }

    pub async fn get_crash_stats(pool: &SqlitePool) -> Result<CrashStats> {
        let rec = sqlx::query!(
            r#"
            SELECT COUNT(*) as total
            FROM crashes
            "#
        )
        .fetch_one(pool)
        .await?;

        Ok(CrashStats {
            total: rec.total as u64,
        })
    }

    pub async fn get_crash_info(guid: &str, pool: &SqlitePool) -> Result<Crash> {
        Ok(sqlx::query_as!(
            Crash,
            r#"
            SELECT guid, name, collection_guid, analyzed, hash, creation_date, size
            FROM crashes
            WHERE guid = $1
            "#,
            guid
        )
        .fetch_one(pool)
        .await?)
    }
}
