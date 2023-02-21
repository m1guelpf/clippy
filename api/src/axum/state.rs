use influxdb2::Client as InfluxDB;
use pika::pika::{InitOptions, Pika, PrefixRecord};
use std::{env, sync::Arc};

use crate::prisma::PrismaClient;

#[derive(Debug)]
pub struct State {
    pub pika: Pika,
    pub influx: InfluxDB,
    pub prisma: PrismaClient,
}

#[allow(clippy::module_name_repetitions)]
pub type AppState = Arc<State>;

pub async fn create(prisma: PrismaClient) -> AppState {
    Arc::new(State {
        prisma,
        pika: get_pika(),
        influx: get_influx().await,
    })
}

fn get_pika() -> Pika {
    let prefixes = vec![
        PrefixRecord {
            prefix: "user".to_string(),
            description: Some("User ID".to_string()),
            secure: false,
        },
        PrefixRecord {
            prefix: "team".to_string(),
            description: Some("Team ID".to_string()),
            secure: false,
        },
        PrefixRecord {
            prefix: "proj".to_string(),
            description: Some("Project ID".to_string()),
            secure: false,
        },
    ];

    Pika::new(prefixes, &InitOptions::default())
}

async fn get_influx() -> InfluxDB {
    let db = InfluxDB::new(
        env::var("INFLUX_HOST").unwrap(),
        env::var("INFLUX_ORG").unwrap(),
        env::var("INFLUX_TOKEN").unwrap(),
    );

    assert!(db.ready().await.unwrap(), "Failed to connect to InfluxDB.");

    db
}
