use async_trait::async_trait;
use chrono::{DateTime, Utc};
use indoc::indoc;
use sqlx::{FromRow, PgPool};
use thiserror::Error;

use crate::{ApiKey, KeyDomain, KeyPool, KeyPoolStorage};

#[derive(Debug, Error)]
pub enum PgStorageError {
    #[error(transparent)]
    Pg(#[from] sqlx::Error),

    #[error("No key avalaible for domain {0:?}")]
    Unavailable(KeyDomain),
}

#[derive(Debug, Clone, FromRow)]
pub struct PgKey {
    pub id: i32,
    pub user_id: i32,
    pub faction_id: Option<i32>,
    pub key: String,
    pub uses: i16,
    pub user: bool,
    pub faction: bool,
    pub last_used: DateTime<Utc>,
}

impl ApiKey for PgKey {
    fn value(&self) -> &str {
        &self.key
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct PgKeyPoolStorage {
    pool: PgPool,
    limit: i16,
}

impl PgKeyPoolStorage {
    pub fn new(pool: PgPool, limit: i16) -> Self {
        Self { pool, limit }
    }

    pub async fn initialise(&self) -> Result<(), PgStorageError> {
        sqlx::query(indoc! {r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id serial primary key,
                user_id int4 not null,
                faction_id int4,
                key char(16) not null,
                uses int2 not null default 0,
                "user" bool not null,
                faction bool not null,
                last_used timestamptz not null default now()
            )"#})
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(feature = "tokio-runtime")]
async fn random_sleep() {
    use rand::{thread_rng, Rng};
    let dur = tokio::time::Duration::from_millis(thread_rng().gen_range(1..50));
    tokio::time::sleep(dur).await;
}

#[cfg(all(not(feature = "tokio-runtime"), feature = "actix-runtime"))]
async fn random_sleep() {
    use rand::{thread_rng, Rng};
    let dur = std::time::Duration::from_millis(thread_rng().gen_range(1..50));
    actix_rt::time::sleep(dur).await;
}

#[async_trait]
impl KeyPoolStorage for PgKeyPoolStorage {
    type Key = PgKey;

    type Error = PgStorageError;

    async fn acquire_key(&self, domain: KeyDomain) -> Result<Self::Key, Self::Error> {
        let predicate = match domain {
            KeyDomain::Public => "".to_owned(),
            KeyDomain::User(id) => format!("where and user_id={} and user", id),
            KeyDomain::Faction(id) => format!("where and faction_id={} and faction", id),
        };

        loop {
            let attempt = async {
                let mut tx = self.pool.begin().await?;

                sqlx::query("set transaction isolation level serializable")
                    .execute(&mut tx)
                    .await?;

                let key: Option<PgKey> = sqlx::query_as(&indoc::formatdoc!(r#"
                    with key as (
                        select 
                            id,
                            user_id,
                            faction_id,
                            key,
                            case
                                when extract(minute from last_used)=extract(minute from now()) then uses
                                else 0::smallint
                            end as uses,
                            user,
                            faction,
                            last_used
                        from api_keys {}
                        order by last_used asc limit 1 FOR UPDATE
                    )
                    update api_keys set
                        uses = key.uses + 1,
                        last_used = now()
                    from key where 
                        api_keys.id=key.id and key.uses < $1
                    returning
                        api_keys.id,
                        api_keys.user_id,
                        api_keys.faction_id,
                        api_keys.key,
                        api_keys.uses,
                        api_keys.user,
                        api_keys.faction,
                        api_keys.last_used
                    "#,
                    predicate
                ))
                .bind(self.limit)
                .fetch_optional(&mut tx)
                .await?;

                tx.commit().await?;

                Result::<Result<Self::Key, Self::Error>, sqlx::Error>::Ok(
                    key.ok_or(PgStorageError::Unavailable(domain)),
                )
            }
            .await;

            match attempt {
                Ok(result) => return result,
                Err(error) => {
                    if let Some(db_error) = error.as_database_error() {
                        let pg_error: &sqlx::postgres::PgDatabaseError = db_error.downcast_ref();
                        if pg_error.code() == "40001" {
                            random_sleep().await;
                        } else {
                            return Err(error.into());
                        }
                    } else {
                        return Err(error.into());
                    }
                }
            }
        }
    }

    async fn flag_key(&self, key: Self::Key, code: u8) -> Result<bool, Self::Error> {
        // TODO: put keys in cooldown when appropriate
        match code {
            2 | 10 | 13 => {
                sqlx::query("delete from api_keys where id=$1")
                    .bind(key.id)
                    .execute(&self.pool)
                    .await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

pub type PgKeyPool<A> = KeyPool<A, PgKeyPoolStorage>;

impl<A> PgKeyPool<A>
where
    A: torn_api::ApiClient,
{
    pub async fn connect(
        client: A,
        database_url: &str,
        limit: i16,
    ) -> Result<Self, PgStorageError> {
        let db_pool = PgPool::connect(database_url).await?;
        let storage = PgKeyPoolStorage::new(db_pool, limit);
        storage.initialise().await?;

        let key_pool = Self::new(client, storage);

        Ok(key_pool)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Once};

    use sqlx::Row;
    use tokio::test;

    use super::*;

    static INIT: Once = Once::new();

    pub(crate) async fn setup() -> PgKeyPoolStorage {
        INIT.call_once(|| {
            dotenv::dotenv().ok();
        });

        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        sqlx::query("update api_keys set uses=0")
            .execute(&pool)
            .await
            .unwrap();

        PgKeyPoolStorage::new(pool, 50)
    }

    #[test]
    async fn test_initialise() {
        let storage = setup().await;

        if let Err(e) = storage.initialise().await {
            panic!("Initialising key storage failed: {:?}", e);
        }
    }

    #[test]
    async fn acquire_one() {
        let storage = setup().await;

        if let Err(e) = storage.acquire_key(KeyDomain::Public).await {
            panic!("Acquiring key failed: {:?}", e);
        }
    }

    #[test]
    async fn test_concurrent() {
        let storage = Arc::new(setup().await);
        let before: i16 = sqlx::query("select uses from api_keys")
            .fetch_one(&storage.pool)
            .await
            .unwrap()
            .get("uses");

        let futures = (0..30).into_iter().map(|_| {
            let storage = storage.clone();
            async move {
                storage.acquire_key(KeyDomain::Public).await.unwrap();
            }
        });
        futures::future::join_all(futures).await;

        let after: i16 = sqlx::query("select uses from api_keys")
            .fetch_one(&storage.pool)
            .await
            .unwrap()
            .get("uses");

        assert_eq!(after, before + 30);
    }
}
