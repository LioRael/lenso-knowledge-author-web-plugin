//! Linked native Web workspace for authoring Knowledge Base drafts.
//!
//! This Plugin owns HTTP presentation and orchestration only. Article, draft,
//! revision, publication, idempotency, and authorization facts remain in the
//! bound `lenso.knowledge-base@1` Provider.

mod assets;

use std::fmt::Debug;

use lenso::prelude::*;
use lenso_auth_sdk::{AuthOutcome, CredentialEvidence, authenticate_request, decode_auth_response};
use lenso_capability_auth as auth;
use lenso_capability_http_endpoint::{
    self as http_endpoint_contract, EndpointHandleInvocationError, ExtractorFuture,
    ExtractorRejection, FromRequest, HandleRequest, HandleResponse, HandleResponseHeadersItem,
    Json, Path, QueryParams, endpoint,
    response::{self, HeaderValue, StatusCode, header},
};
use lenso_capability_knowledge_base as knowledge_base;
use lenso_kernel::{InvocationContext, RuntimeFailure};
use serde::{Deserialize, Serialize};

/// Forces this native Plugin crate to be retained by a linked Host.
pub const fn link() {}

#[lenso::plugin]
#[derive(Clone, Debug, Default)]
pub struct KnowledgeAuthorWebPlugin {
    auth: Port<auth::AuthClient>,
    knowledge_base: Port<knowledge_base::KnowledgeBaseClient>,
}

#[endpoint]
impl KnowledgeAuthorWebPlugin {
    #[get("knowledge.author.web.page", "/knowledge")]
    async fn page(&self) -> Result<HandleResponse, EndpointHandleInvocationError> {
        std::future::ready(()).await;
        Ok(asset(
            StatusCode::OK,
            "text/html; charset=utf-8",
            assets::PAGE,
        ))
    }

    #[get("knowledge.author.web.css", "/knowledge/assets/app.css")]
    async fn css(&self) -> Result<HandleResponse, EndpointHandleInvocationError> {
        std::future::ready(()).await;
        Ok(asset(
            StatusCode::OK,
            "text/css; charset=utf-8",
            assets::CSS,
        ))
    }

    #[get("knowledge.author.web.js", "/knowledge/assets/app.js")]
    async fn javascript(&self) -> Result<HandleResponse, EndpointHandleInvocationError> {
        std::future::ready(()).await;
        Ok(asset(
            StatusCode::OK,
            "text/javascript; charset=utf-8",
            assets::JS,
        ))
    }

    #[get("knowledge.author.web.articles.list", "/api/knowledge/articles")]
    async fn list_articles(
        &self,
        _author: AuthenticatedAuthor,
        context: InvocationContext,
        QueryParams(query): QueryParams<ListArticlesQuery>,
    ) -> Result<HandleResponse, EndpointHandleInvocationError> {
        json_result(
            self.knowledge_base
                .list_articles_with_context(context, query.into_request())
                .await,
            StatusCode::OK,
        )
    }

    #[post("knowledge.author.web.articles.create", "/api/knowledge/articles")]
    async fn create_draft(
        &self,
        _author: AuthenticatedAuthor,
        context: InvocationContext,
        Json(request): Json<knowledge_base::CreateDraftRequest>,
    ) -> Result<HandleResponse, EndpointHandleInvocationError> {
        json_result(
            self.knowledge_base
                .create_draft_with_context(context, request)
                .await,
            StatusCode::CREATED,
        )
    }

    #[get(
        "knowledge.author.web.articles.get",
        "/api/knowledge/articles/{article_id}"
    )]
    async fn get_draft(
        &self,
        _author: AuthenticatedAuthor,
        context: InvocationContext,
        Path(path): Path<ArticlePath>,
        QueryParams(query): QueryParams<OrganizationQuery>,
    ) -> Result<HandleResponse, EndpointHandleInvocationError> {
        json_result(
            self.knowledge_base
                .get_draft_with_context(
                    context,
                    knowledge_base::GetDraftRequest {
                        article_id: path.article_id,
                        organization_id: query.organization_id,
                    },
                )
                .await,
            StatusCode::OK,
        )
    }

    #[patch(
        "knowledge.author.web.articles.update",
        "/api/knowledge/articles/{article_id}"
    )]
    async fn update_draft(
        &self,
        _author: AuthenticatedAuthor,
        context: InvocationContext,
        Path(path): Path<ArticlePath>,
        Json(request): Json<knowledge_base::UpdateDraftRequest>,
    ) -> Result<HandleResponse, EndpointHandleInvocationError> {
        if path.article_id != request.article_id {
            return Ok(path_mismatch());
        }
        json_result(
            self.knowledge_base
                .update_draft_with_context(context, request)
                .await,
            StatusCode::OK,
        )
    }

    #[post(
        "knowledge.author.web.articles.publish",
        "/api/knowledge/articles/{article_id}/publish"
    )]
    async fn publish_article(
        &self,
        _author: AuthenticatedAuthor,
        context: InvocationContext,
        Path(path): Path<ArticlePath>,
        Json(request): Json<knowledge_base::PublishArticleRequest>,
    ) -> Result<HandleResponse, EndpointHandleInvocationError> {
        if path.article_id != request.article_id {
            return Ok(path_mismatch());
        }
        json_result(
            self.knowledge_base
                .publish_article_with_context(context, request)
                .await,
            StatusCode::OK,
        )
    }
}

#[derive(Debug)]
struct AuthenticatedAuthor;

impl FromRequest<KnowledgeAuthorWebPlugin> for AuthenticatedAuthor {
    fn from_request<'a>(
        provider: &'a KnowledgeAuthorWebPlugin,
        context: &'a mut InvocationContext,
        request: &'a HandleRequest,
    ) -> ExtractorFuture<'a, Self> {
        Box::pin(async move {
            let evidence = request
                .credential
                .as_ref()
                .map(|credential| CredentialEvidence::new(&credential.scheme, &credential.value));
            let response = provider
                .auth
                .authenticate_with_context(context.clone(), authenticate_request(evidence))
                .await
                .map_err(|error| -> ExtractorRejection {
                    match error {
                        auth::AuthInvocationError::Domain(_) => authentication_problem().into(),
                        auth::AuthInvocationError::Runtime(error) => {
                            EndpointHandleInvocationError::Runtime(error).into()
                        }
                    }
                })?;
            let outcome = decode_auth_response(response).map_err(|_| {
                EndpointHandleInvocationError::Runtime(RuntimeFailure::ProtocolViolation {
                    capability: auth::CAPABILITY_ID,
                })
            })?;
            let AuthOutcome::Authenticated(assertion) = outcome else {
                return Err(authentication_problem().into());
            };
            if assertion.actor_kind() != "user" {
                return Err(response::problem(
                    StatusCode::FORBIDDEN,
                    "unsupported_actor",
                    "This author workspace requires an authenticated user actor.",
                )
                .into());
            }
            *context = assertion.attach(context.clone()).map_err(|error| {
                EndpointHandleInvocationError::Runtime(RuntimeFailure::Internal {
                    detail: format!("could not attach authenticated author assertion: {error}"),
                })
            })?;
            Ok(Self)
        })
    }
}

fn authentication_problem() -> HandleResponse {
    response::problem(
        StatusCode::UNAUTHORIZED,
        "authentication_required",
        "Provide a valid Bearer credential.",
    )
    .with_header(
        &header::WWW_AUTHENTICATE,
        &HeaderValue::from_static("Bearer"),
    )
    .expect("the static WWW-Authenticate header is valid")
}

fn asset(status: StatusCode, content_type: &str, body: &str) -> HandleResponse {
    HandleResponse {
        body: body.as_bytes().to_vec().into(),
        headers: vec![HandleResponseHeadersItem {
            name: "content-type".to_owned(),
            value: content_type.to_owned(),
        }],
        status: i64::from(status.as_u16()),
    }
}

fn path_mismatch() -> HandleResponse {
    response::problem(
        StatusCode::BAD_REQUEST,
        "path_body_mismatch",
        "The path and JSON body must name the same article_id.",
    )
}

trait IntoWebError {
    fn into_web_error(self) -> Result<HandleResponse, EndpointHandleInvocationError>;
}

fn json_result<T, E>(
    result: Result<T, E>,
    status: StatusCode,
) -> Result<HandleResponse, EndpointHandleInvocationError>
where
    T: Serialize,
    E: IntoWebError,
{
    match result {
        Ok(value) => response::json(status, &value).map_err(Into::into),
        Err(error) => error.into_web_error(),
    }
}

fn domain_problem(error: &impl Debug) -> Result<HandleResponse, EndpointHandleInvocationError> {
    let variant = format!("{error:?}");
    if variant.starts_with("Unknown(") {
        return Err(EndpointHandleInvocationError::Runtime(
            RuntimeFailure::ProtocolViolation {
                capability: knowledge_base::CAPABILITY_ID,
            },
        ));
    }
    let code = snake_case(&variant);
    let status = match variant.as_str() {
        "Unauthenticated" => StatusCode::UNAUTHORIZED,
        "Forbidden" => StatusCode::FORBIDDEN,
        "ArticleNotFound" => StatusCode::NOT_FOUND,
        "SlugConflict" | "IdempotencyConflict" | "RevisionConflict" => StatusCode::CONFLICT,
        _ => StatusCode::BAD_REQUEST,
    };
    let mut problem = response::problem(
        status,
        code.clone(),
        format!("The Knowledge Base capability rejected this operation ({code})."),
    );
    if status == StatusCode::UNAUTHORIZED {
        problem = problem.with_header(
            &header::WWW_AUTHENTICATE,
            &HeaderValue::from_static("Bearer"),
        )?;
    }
    Ok(problem)
}

fn snake_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() && index != 0 {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}

macro_rules! impl_web_error {
    ($type:path) => {
        impl IntoWebError for $type {
            fn into_web_error(self) -> Result<HandleResponse, EndpointHandleInvocationError> {
                match self {
                    Self::Domain(error) => domain_problem(&error),
                    Self::Runtime(error) => Err(EndpointHandleInvocationError::Runtime(error)),
                }
            }
        }
    };
}

impl_web_error!(knowledge_base::KnowledgeBaseCreateDraftInvocationError);
impl_web_error!(knowledge_base::KnowledgeBaseGetDraftInvocationError);
impl_web_error!(knowledge_base::KnowledgeBaseListArticlesInvocationError);
impl_web_error!(knowledge_base::KnowledgeBasePublishArticleInvocationError);
impl_web_error!(knowledge_base::KnowledgeBaseUpdateDraftInvocationError);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArticlePath {
    article_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrganizationQuery {
    organization_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArticlesQuery {
    organization_id: String,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

impl ListArticlesQuery {
    fn into_request(self) -> knowledge_base::ListArticlesRequest {
        knowledge_base::ListArticlesRequest {
            cursor: self.cursor,
            limit: self.limit,
            organization_id: self.organization_id,
        }
    }
}

const fn default_limit() -> i64 {
    40
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use lenso_capability_http_endpoint::testing::EndpointTest;

    use super::*;

    #[test]
    fn serves_self_contained_author_workspace_assets() {
        block_on(async {
            let endpoint = EndpointTest::new(KnowledgeAuthorWebPlugin::default());
            let page = endpoint
                .request("knowledge.author.web.page")
                .send()
                .await
                .unwrap();
            assert_eq!(page.status(), StatusCode::OK);
            assert_eq!(
                page.header("content-type"),
                Some("text/html; charset=utf-8")
            );
            assert!(page.into_inner().body.starts_with(b"<!doctype html>"));

            let css = endpoint
                .request("knowledge.author.web.css")
                .send()
                .await
                .unwrap();
            assert_eq!(css.header("content-type"), Some("text/css; charset=utf-8"));

            let javascript = endpoint
                .request("knowledge.author.web.js")
                .send()
                .await
                .unwrap();
            assert_eq!(
                javascript.header("content-type"),
                Some("text/javascript; charset=utf-8")
            );
            let api_path = b"/api/knowledge/articles";
            assert!(
                javascript
                    .into_inner()
                    .body
                    .windows(api_path.len())
                    .any(|chunk| chunk == api_path)
            );
        });
    }

    #[test]
    fn descriptor_declares_only_auth_and_knowledge_base_requirements() {
        let descriptor: serde_json::Value = serde_json::from_str(PLUGIN_DESCRIPTOR_JSON).unwrap();
        let provided = descriptor["provided_capabilities"].as_array().unwrap();
        assert_eq!(provided.len(), 1);
        assert_eq!(provided[0]["capability_id"], "lenso.http.endpoint@1");
        assert_eq!(provided[0]["descriptor_version"], "1.1.0");

        let mut required = descriptor["required_capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry["capability_id"].as_str().unwrap(),
                    entry["descriptor_version"].as_str().unwrap(),
                    entry["cardinality"].as_str().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        required.sort_unstable();
        assert_eq!(
            required,
            vec![
                ("lenso.auth@1", "1.0.0", "one"),
                ("lenso.knowledge-base@1", "1.1.0", "one"),
            ]
        );
    }

    #[test]
    fn maps_authorization_and_optimistic_concurrency_explicitly() {
        assert_eq!(
            domain_problem(&knowledge_base::ListArticlesError::Forbidden)
                .unwrap()
                .status,
            403
        );
        assert_eq!(
            domain_problem(&knowledge_base::UpdateDraftError::RevisionConflict)
                .unwrap()
                .status,
            409
        );
        assert_eq!(
            domain_problem(&knowledge_base::GetDraftError::ArticleNotFound)
                .unwrap()
                .status,
            404
        );
    }
}
