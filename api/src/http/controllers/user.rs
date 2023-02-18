use axum_jsonschema::Json;

use crate::{axum::extractors::User, prisma::user};

#[allow(clippy::unused_async)]
pub async fn show(User(user): User) -> Json<user::Data> {
    Json(user)
}
