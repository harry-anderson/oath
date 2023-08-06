use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemOutput;
use aws_sdk_dynamodb::operation::put_item::PutItemOutput;
use aws_sdk_dynamodb::types::AttributeValue;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_dynamo::aws_sdk_dynamodb_0_28::{to_attribute_value, to_item};
use serde_dynamo::from_items;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct DbClient {
    table_name: String,
    inner: Client,
}

impl DbClient {
    pub async fn new(table_name: &str) -> Arc<Self> {
        let shared_config = aws_config::load_from_env().await;

        Arc::new(Self {
            inner: Client::new(&shared_config),
            table_name: table_name.to_string(),
        })
    }

    pub async fn put(
        &self,
        item: impl Serialize + std::fmt::Debug,
    ) -> anyhow::Result<PutItemOutput> {
        let item = to_item(item)?;
        self.inner
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    pub async fn delete(&self, pk: String, sk: String) -> anyhow::Result<DeleteItemOutput> {
        self.inner
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", to_attribute_value(pk)?)
            .key("SK", to_attribute_value(sk)?)
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    pub async fn query<T>(
        &self,
        key_condition_expression: &str,
        expression_attribute_names: HashMap<String, String>,
        expression_attribute_values: HashMap<String, AttributeValue>,
        gsi: Option<String>,
    ) -> anyhow::Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let res = self
            .inner
            .query()
            .table_name(&self.table_name)
            .key_condition_expression(key_condition_expression)
            .set_expression_attribute_names(Some(expression_attribute_names))
            .set_expression_attribute_values(Some(expression_attribute_values))
            .set_index_name(gsi)
            .send()
            .await?;

        let items = res.items.unwrap_or(vec![]);
        let v: Vec<T> = from_items(items.to_vec())?;
        Ok(v)
    }

    pub async fn query_single_table<T>(
        &self,
        pk: String,
        sk: Option<String>,
        gsi: Option<String>,
    ) -> anyhow::Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let (primary_index, secondary_index) = match gsi {
            Some(_) => ("SK", "PK"),
            None => ("PK", "SK"),
        };
        match sk {
            Some(sk) => {
                self.query(
                    "#pk = :pk and #sk = :sk",
                    HashMap::from([
                        (String::from("#pk"), String::from(primary_index)),
                        (String::from("#sk"), String::from(secondary_index)),
                    ]),
                    HashMap::from([
                        (String::from(":pk"), AttributeValue::S(pk)),
                        (String::from(":sk"), AttributeValue::S(sk)),
                    ]),
                    gsi,
                )
                .await
            }
            None => {
                self.query(
                    "#pk = :pk",
                    HashMap::from([(String::from("#pk"), String::from(primary_index))]),
                    HashMap::from([(String::from(":pk"), AttributeValue::S(pk))]),
                    gsi,
                )
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_dynamo::aws_sdk_dynamodb_0_19::to_attribute_value;
    use serde_json::{json, to_string};
    use uuid::Uuid;

    #[tokio::test]
    async fn pk() {
        let client = DbClient::new("oath-db-dev").await;
        let res = client
            .query::<Entity>(
                "#pk = :pk",
                HashMap::from([(String::from("#pk"), String::from("PK"))]),
                HashMap::from([(
                    String::from(":pk"),
                    to_attribute_value(String::from("u#harry@gmail.com")).unwrap(),
                )]),
                None,
            )
            .await
            .unwrap();

        println!("{:#?}", res)
    }
}
