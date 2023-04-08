use anyhow::Context;
use axum::extract::Path;
use axum::extract::State;
use axum_jsonschema::Json;
use axum_yaml::Yaml;
use opg::{describe_api, HttpMethod, OpgModel, Path as OpgPath, PathElement};
use prisma_client_rust::operator;
use schemars::JsonSchema;
use std::env;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        state::AppState,
    },
    http::controllers::widget::AskRequest,
    prisma::project,
    utils::influx,
};
use ::clippy::{search_project, Payload};

#[derive(serde::Serialize, OpgModel)]
pub struct SearchResponse {
    results: Vec<Payload>,
    base_url: String,
}

pub async fn search(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(AskRequest { query }): Json<AskRequest>,
) -> ApiResult<Json<SearchResponse>> {
    let params = name
        .to_lowercase()
        .split(' ')
        .filter(|n| !vec!["docs", "documentation"].contains(n))
        .flat_map(|n| {
            vec![
                project::name::contains(n.to_string()),
                project::index_name::contains(n.to_string()),
                project::origins::string_contains(n.to_string()),
            ]
        })
        .collect::<Vec<_>>();

    let Ok(Some(project)) = state
        .prisma
        .project()
        .find_first(vec![operator::or(params)])
        .exec()
        .await
     else {
        return Err(ApiError::ProjectNotFound)
    };

    influx::track_search(&state.influx, &project.id)
        .await
        .context("Failed to track widget search.")?;

    let results = search_project(
        &project
            .index_name
            .expect("Trained models should have an index set."),
        &query,
        5,
    )
    .await
    .context("Failed to search project.")?;

    let origins: Vec<String> =
        serde_json::from_value(project.origins).context("Failed to parse origins as JSON.")?;

    Ok(Json(SearchResponse {
        results: results.into_iter().map(|r| r.payload).collect(),
        base_url: origins[0].clone(),
    }))
}

#[derive(serde::Serialize, JsonSchema)]
struct ApiSpec<'a> {
    url: String,
    r#type: &'a str,
    is_user_authenticated: bool,
}

#[derive(serde::Serialize, JsonSchema)]
struct ManifestAuth<'a> {
    r#type: &'a str,
}

#[derive(serde::Serialize, JsonSchema)]
pub struct Manifest<'a> {
    schema_version: &'a str,
    name_for_human: &'a str,
    name_for_model: &'a str,
    description_for_human: &'a str,
    description_for_model: &'a str,
    auth: ManifestAuth<'a>,
    api: ApiSpec<'a>,
    logo_url: &'a str,
    contact_email: &'a str,
    legal_info_url: &'a str,
}

#[allow(clippy::unused_async)]
pub async fn manifesto() -> Json<Manifest<'static>> {
    let app_url = env::var("APP_URL").unwrap();

    Json(Manifest {
        schema_version: "v1",
        name_for_human: "Clippy",
        name_for_model: "clippy",
        description_for_human: "Upgrades ChatGPT with the latest documentation from your favorite projects.",
        description_for_model: "Use the Clippy plugin when the user asks a question related to any of the following projects: Worldcoin (or World ID), Hop, ENS Domains, Next.js, Whop, Bun, or BuilderBook. Do not output the information returned literally, use it as additional context you might use when crafting your response.",
        auth: ManifestAuth { r#type: "none" },
        api: ApiSpec { r#type: "openapi", url: format!("{app_url}/.well-known/chatgpt-api.yaml"), is_user_authenticated: false },
        logo_url: "https://i.imgur.com/zBxRq0S.jpg",
        contact_email: "chatgpt@clippy.help",
        legal_info_url: "https://example.com/legal"
    })
}

#[allow(clippy::unused_async)]
pub async fn openapi() -> Yaml<opg::Opg> {
    let api_url = env::var("APP_URL").unwrap();

    #[allow(clippy::needless_update)]
    let mut schema = describe_api! {
        info: {
            title: "Clippy API for ChatGPT",
            description: "A plugin that retrieves up-to-date content from project documentation using semantic search.",
            version: "0.0.0",
        },
        servers: { api_url },
        paths: {
            ("chatgpt" / "search" / { project: String }): {
                POST: {
                    operationId: "search",
                    summary: "Searches a project's documentation for a given query.",
                    description: "Searches a project's documentation for a given query.",
                    body: {
                        schema: AskRequest,
                        required: true
                    },
                    200: SearchResponse,
                    404: String
                }
            },
        }
    };

    for (path, path_value) in &mut schema.paths {
        if !has_parameter(path, &PathElement::Parameter("project".to_string())) {
            continue;
        }

        let mut project_id = path_value.parameters.get("project").unwrap().clone();
        project_id.description = Some("The name of the project to search for.".to_string());

        path_value.parameters.clear();
        path_value
            .operations
            .entry(HttpMethod::POST)
            .and_modify(|method| {
                method.parameters.insert("project".to_string(), project_id);
            })
            .or_default();
    }

    Yaml(schema)
}

fn has_parameter(path: &OpgPath, parameter: &PathElement) -> bool {
    for element in &path.0 {
        if format!("{element:?}") == format!("{parameter:?}") {
            return true;
        }
    }

    false
}
