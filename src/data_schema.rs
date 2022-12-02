use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThingSchema {
    pub id: String,
    #[serde(rename = "@type")]
    pub r#type: Option<Either<String, Vec<String>>>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub properties: BTreeMap<String, DataSchema>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataSchema {
    pub id: String,
    #[serde(rename = "@type")]
    pub r#type: Option<Either<String, Vec<String>>>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "const")]
    pub r#const: Value,
    pub unit: Option<String>,
    pub one_of: Option<Vec<DataSchema>>,
    pub read_only: bool,
    pub write_only: bool,
    pub format: Option<String>,
    #[serde(flatten)]
    pub detail: DetailDataSchema,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(tag = "type")]
pub enum DetailDataSchema {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "number")]
    Number {
        minimum: Option<f64>,
        maximum: Option<f64>,
    },
    #[serde(rename = "integer")]
    Integer {
        minimum: Option<i64>,
        maximum: Option<i64>,
    },

    #[serde(rename = "string")]
    String,

    #[serde(rename = "null")]
    #[default]
    Null,

    #[serde(rename = "object")]
    Object {
        properties: BTreeMap<String, DataSchema>,
        required: Vec<String>,
    },

    #[serde(rename = "array")]
    Array {
        items: Vec<DataSchema>,
        min_items: u32,
        max_items: u32,
    },
}

pub trait Schema {
    fn get_schema(&self) -> BTreeMap<String, DataSchema>;
}
