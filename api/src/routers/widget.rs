use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::StreamExt;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::{Origin, Project},
        state::AppState,
    },
    prisma::{self, project},
};
use ::clippy::stream::PartialResult;

pub fn mount() -> Router<AppState> {
    Router::new()
        .route("/widget", get(widget_info))
        .route("/:project/stream", post(stream))
}

async fn widget_info(
    Origin(origin): Origin,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    let project = state
        .prisma
        .project()
        .find_first(vec![project::WhereParam::Origins(
            prisma::read_filters::JsonFilter::ArrayContains(Some(origin.into())),
        )])
        .select(project::select! ({ id image_url copy }))
        .exec()
        .await;

    let Ok(Some(project)) = project else {
        return Err(ApiError::ProjectNotFound);
    };

    Ok(Json(serde_json::to_value(project).unwrap()))
}

#[derive(Debug, serde::Deserialize)]
struct AskRequest {
    query: String,
}

#[allow(clippy::unused_async)]
async fn stream(
    Project(project): Project,
    Json(req): Json<AskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = clippy::stream::ask(project.index_name, req.query);

    let stream = stream.map(|e| {
        let Ok(event) = e else {
            return Ok::<_, Infallible>(Event::default().id("error").json_data(json!({
                "error": "Failed to complete query."
            })).unwrap())
        };

        match event {
            PartialResult::References(results) => {
                let results = results
                    .into_iter()
                    .map(|p| {
                        json!({
                            "path": p.path,
                            "text": p.text,
                            "title": p.title,
                            "page": p.page_title,
                        })
                    })
                    .collect::<Vec<Value>>();

                Ok::<_, Infallible>(
                    Event::default()
                        .id("references")
                        .json_data(results)
                        .unwrap(),
                )
            }
            PartialResult::Answer(answer) => {
                Ok::<_, Infallible>(Event::default().id("answer").json_data(answer).unwrap())
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
