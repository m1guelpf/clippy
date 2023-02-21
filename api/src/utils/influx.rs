use std::env;

use anyhow::Result;
use futures::stream;
use influxdb2::{models::DataPoint, Client};

const WIDGET_QUERY_MEASUREMENT: &str = "widget_query";
const WIDGET_LOAD_MEASUREMENT: &str = "widget_load";
const WIDGET_SEARCH_MEASUREMENT: &str = "widget_search";

pub async fn track_load(client: &Client, project_id: &str) -> Result<()> {
    track_event(client, project_id, WIDGET_LOAD_MEASUREMENT).await
}

pub async fn track_search(client: &Client, project_id: &str) -> Result<()> {
    track_event(client, project_id, WIDGET_SEARCH_MEASUREMENT).await
}

pub async fn track_query(client: &Client, project_id: &str) -> Result<()> {
    track_event(client, project_id, WIDGET_QUERY_MEASUREMENT).await
}

async fn track_event(client: &Client, project_id: &str, event: &str) -> Result<()> {
    let point = DataPoint::builder(event)
        .tag("project_id", project_id)
        .field("value", 1)
        .build()?;

    Ok(client
        .write(
            &env::var("INFLUX_DB").unwrap(),
            stream::once(async { point }),
        )
        .await?)
}
