use async_session::{async_trait, Result, Session, SessionStore};
use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::{collections::HashMap, sync::Arc};

use crate::aws::dynamodb::DbClient;

const SESSION_PK: &str = "SESSION";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DynamoSession {
    #[serde(rename = "PK")]
    pk: String,
    #[serde(rename = "SK")]
    sk: String,
    session: String,
}

impl DynamoSession {
    pub fn new(id: &str, session: String) -> Self {
        Self {
            pk: String::from(SESSION_PK),
            sk: id.to_string(),
            session,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynamoSessionStore {
    db: Arc<DbClient>,
}

impl DynamoSessionStore {
    pub async fn new(db: Arc<DbClient>) -> DynamoSessionStore {
        Self { db }
    }
}

#[async_trait]
impl SessionStore for DynamoSessionStore {
    // query -> pk SESSION sk 1234
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        let id = Session::id_from_cookie_value(&cookie_value)?;
        tracing::info!("loading session by id `{}`", id);

        match self
            .db
            .query_single_table::<DynamoSession>(SESSION_PK.to_string(), Some(id), None)
            .await
        {
            Err(err) => {
                tracing::error!("load session error {}", err);
                Err(async_session::Error::msg(format!(
                    "load session error {}",
                    err
                )))
            }
            Ok(res) => match res.into_iter().next() {
                None => {
                    tracing::info!("load session not found");
                    Ok(None)
                }
                Some(dbs) => match from_str::<Session>(&dbs.session) {
                    Err(err) => {
                        tracing::error!("load session error {}", err);
                        Err(async_session::Error::msg(format!(
                            "load session error {}",
                            err
                        )))
                    }
                    Ok(s) => Ok(Session::validate(s)),
                },
            },
        }
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        tracing::info!("storing session by id `{}`", session.id());
        let session_json = to_string(&session)?;
        let db_session = DynamoSession::new(session.id(), session_json);

        match self.db.put(&db_session).await {
            Err(err) => {
                tracing::error!("store session error {:?}", err);
                Err(async_session::Error::msg(format!(
                    "store session error {}",
                    err
                )))
            }
            Ok(res) => {
                tracing::info!("store session success {:?}", res);
                session.reset_data_changed();
                Ok(session.into_cookie_value())
            }
        }
    }

    async fn destroy_session(&self, session: Session) -> Result {
        tracing::info!("destroying session by id `{}`", session.id());
        match self
            .db
            .delete(String::from(SESSION_PK), session.id().to_string())
            .await
        {
            Err(err) => {
                tracing::error!("store session error {:?}", err);
                Err(async_session::Error::msg(format!(
                    "store session error {}",
                    err
                )))
            }
            Ok(res) => {
                tracing::info!("delete session success {:?}", res);
                Ok(())
            }
        }
    }

    async fn clear_store(&self) -> Result {
        tracing::info!("clearing session store");

        match self.db
            .query::<DynamoSession>(
                "#pk = :pk",
                HashMap::from([(String::from("#pk"), String::from("PK"))]),
                HashMap::from([(
                    String::from(":pk"),
                    AttributeValue::S(SESSION_PK.to_string()),
                )]),
                None,
            )
            .await
        {
            Err(err) => {
                tracing::error!("load session error {:?}", err);
                Err(async_session::Error::msg(format!(
                    "load session error {}",
                    err
                )))
            }
            Ok(db_sessions) => {
                for item in db_sessions {
                    //TODO join futures
                    if let Err(err) = self.db.delete(item.pk, item.sk).await {
                        tracing::error!("failed to delete session {:?}", err);
                    }
                }
                Ok(())
            }
        }
    }
}
