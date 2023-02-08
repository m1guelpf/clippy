use pika::pika::{InitOptions, Pika, PrefixRecord};
use std::sync::Arc;

use crate::prisma::PrismaClient;

#[derive(Debug)]
pub struct State {
    pub pika: Pika,
    pub prisma: PrismaClient,
}

#[allow(clippy::module_name_repetitions)]
pub type AppState = Arc<State>;

pub fn create(prisma: PrismaClient) -> AppState {
    Arc::new(State {
        prisma,
        pika: get_pika(),
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
