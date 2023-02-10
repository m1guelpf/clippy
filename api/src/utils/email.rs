use postmark::{
    api::email::{SendEmailWithTemplateRequest, SendEmailWithTemplateRequestBuilder},
    reqwest::PostmarkClient,
    Query,
};
use serde::Serialize;
use std::{collections::HashMap, env};

pub async fn send<Q>(req: Q) -> Q::Result
where
    Q: Query<PostmarkClient> + Send,
{
    let client = PostmarkClient::builder()
        .token(env::var("POSTMARK_TOKEN").unwrap())
        .build();

    req.execute(&client).await
}

#[allow(clippy::type_complexity)]
pub fn from_template<S, V>(
    template_id: S,
    params: HashMap<S, V>,
) -> SendEmailWithTemplateRequestBuilder<(
    (String,),
    (),
    (),
    (Option<String>,),
    (postmark::api::email::TemplateModel,),
    (),
    (),
    (),
    (),
    (),
    (),
    (),
    (),
    (),
    (),
)>
where
    S: Into<String>,
    V: Serialize,
{
    SendEmailWithTemplateRequest::builder()
        .from(env::var("MAIL_FROM").unwrap())
        .template_alias(template_id)
        .template_model(params)
}
