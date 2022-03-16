use crate::protos::agent::CrashMsg;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow, Default)]
pub struct Crash {
    pub guid: String,
    pub name: String,
    pub collection_guid: String,
    pub analyzed: Option<String>,
}

impl Crash {
    pub async fn new_crash(crash: &CrashMsg, pool: &SqlitePool) -> Result<()> {
        let guid = Uuid::new_v4().to_string();
        sqlx::query!(
            r#"
            INSERT INTO crashes (guid, name, collection_guid, analyzed)
            VALUES($1, $2, $3, NULL)
            "#,
            guid,
            crash.name,
            crash.job_guid,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_crashes(pool: &SqlitePool) -> Result<Vec<Crash>> {
        Ok(sqlx::query_as!(
            Crash,
            r#"
            SELECT guid, name, collection_guid, analyzed
            FROM crashes
            "#
        )
        .fetch_all(pool)
        .await?)
    }
}
