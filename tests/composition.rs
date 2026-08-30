use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration as StdDuration};

use lenso_app_plan::{
    AppComposition, CapabilityBinding, CapabilityEndpointPlan, CapabilityRequirementPlan,
    PluginInstancePlan, ResolvedAppPlan,
};
use lenso_auth_sdk::{
    ActorAssertion, ActorAssertionIssuer, ActorProjectionError, FixedClock, TypedActor, Validity,
    audience, authenticated_response,
};
use lenso_capability_auth as auth;
use lenso_capability_auth::{Auth, AuthEndpoint, AuthProvider};
use lenso_capability_http_endpoint as endpoint;
use lenso_capability_http_endpoint::{
    HandleRequest, HandleRequestCredential, HandleRequestHeadersItem,
    HandleRequestPathParametersItem,
};
use lenso_capability_knowledge_base as knowledge_base;
use lenso_kernel::{
    InvocationContext, Kernel, NativeRequestFuture, RuntimeFailure, ShutdownOutcome,
};
use lenso_knowledge_author_web_plugin::PACKAGE_ID;
use lenso_native_adapter::{
    NativePluginFactory, NativePluginFactoryContext, NativePluginInstance, NativePluginRegistry,
};
use lenso_runner::TokioDriver;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use time::{Duration, OffsetDateTime};

const CALLER_PACKAGE: &str = "test.knowledge-author-caller";
const AUTH_PACKAGE: &str = "test.knowledge-author-auth";
const DOMAIN_PACKAGE: &str = "test.knowledge-author-domain";
const NOW: &str = "2027-01-15T08:00:00Z";

const KNOWLEDGE_OPERATIONS: &[&str] = &[
    knowledge_base::CREATE_DRAFT_OPERATION,
    knowledge_base::GET_DRAFT_OPERATION,
    knowledge_base::GET_PUBLISHED_ARTICLE_OPERATION,
    knowledge_base::LIST_ARTICLES_OPERATION,
    knowledge_base::PUBLISH_ARTICLE_OPERATION,
    knowledge_base::SEARCH_PUBLISHED_ARTICLES_OPERATION,
    knowledge_base::UPDATE_DRAFT_OPERATION,
];

#[derive(Clone, Copy, Debug)]
enum KnowledgeMode {
    Success,
    Forbidden,
    Runtime,
}

#[tokio::test(flavor = "current_thread")]
async fn author_can_create_list_reload_update_and_publish_with_auth_and_conflict_traces() {
    tokio::task::LocalSet::new()
        .run_until(async {
            lenso_knowledge_author_web_plugin::link();
            let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
            let issuer = ActorAssertionIssuer::new("test.auth", b"knowledge-author-test-key");
            let draft = Rc::new(RefCell::new(None));
            let app = start_web_app(issuer, now, Rc::clone(&draft), KnowledgeMode::Success).await;

            let unauthenticated = invoke(&app, list_request("bad")).await;
            assert_eq!(unauthenticated.status, 401);
            assert!(body_text(&unauthenticated).contains("authentication_required"));

            let unsupported_actor = invoke(&app, list_request("wrong-kind")).await;
            assert_eq!(unsupported_actor.status, 403);
            assert!(body_text(&unsupported_actor).contains("unsupported_actor"));

            let created = invoke(&app, create_request("good")).await;
            assert_eq!(created.status, 201);
            assert_eq!(body_json(&created)["article_id"], "article_1");
            assert_eq!(body_json(&created)["revision"], "rev-1");

            let listed = invoke(&app, list_request("good")).await;
            assert_eq!(listed.status, 200);
            let listed_body = body_json(&listed);
            assert_eq!(listed_body["articles"][0]["article_id"], "article_1");
            assert_eq!(listed_body["articles"][0]["title"], "Reset your password");

            let reloaded = invoke(&app, get_request("good")).await;
            assert_eq!(reloaded.status, 200);
            assert_eq!(
                body_json(&reloaded)["body_markdown"],
                "# Reset your password"
            );
            assert_eq!(body_json(&reloaded)["revision"], "rev-1");

            let updated = invoke(&app, update_request("good", "rev-1", "update-1")).await;
            assert_eq!(updated.status, 200);
            assert_eq!(body_json(&updated)["title"], "Reset a forgotten password");
            assert_eq!(body_json(&updated)["revision"], "rev-2");

            let stale = invoke(&app, update_request("good", "rev-1", "update-stale")).await;
            assert_eq!(stale.status, 409);
            assert!(body_text(&stale).contains("revision_conflict"));

            let published = invoke(&app, publish_request("good", "rev-2")).await;
            assert_eq!(published.status, 200);
            assert_eq!(body_json(&published)["article_revision"], "rev-2");
            assert_eq!(
                body_json(&published)["publication_revision"],
                "publication-1"
            );

            let after_publish = invoke(&app, get_request("good")).await;
            assert_eq!(
                body_json(&after_publish)["latest_published_article_revision"],
                "rev-2"
            );
            assert_eq!(draft.borrow().as_ref().unwrap().revision, 2);

            assert_eq!(
                app.shutdown(StdDuration::from_secs(1)).await,
                ShutdownOutcome::Clean
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn final_knowledge_authorization_denial_is_403_and_runtime_failure_is_unchanged() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
            let issuer = ActorAssertionIssuer::new("test.auth", b"knowledge-author-test-key");

            let forbidden = start_web_app(
                issuer.clone(),
                now,
                Rc::new(RefCell::new(None)),
                KnowledgeMode::Forbidden,
            )
            .await;
            let response = invoke(&forbidden, list_request("good")).await;
            assert_eq!(response.status, 403);
            assert!(body_text(&response).contains("forbidden"));
            assert_eq!(
                forbidden.shutdown(StdDuration::from_secs(1)).await,
                ShutdownOutcome::Clean
            );

            let unavailable = start_web_app(
                issuer,
                now,
                Rc::new(RefCell::new(None)),
                KnowledgeMode::Runtime,
            )
            .await;
            assert!(matches!(
                unavailable
                    .invoke::<endpoint::EndpointHandle>(
                        "caller",
                        endpoint::HANDLE_OPERATION,
                        list_request("good"),
                    )
                    .await,
                Err(RuntimeFailure::Unavailable {
                    capability: knowledge_base::CAPABILITY_ID
                })
            ));
            assert_eq!(
                unavailable.shutdown(StdDuration::from_secs(1)).await,
                ShutdownOutcome::Clean
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn removing_author_workspace_keeps_knowledge_base_invocable() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
            let issuer = ActorAssertionIssuer::new("test.auth", b"knowledge-author-test-key");
            let app = Kernel::start_native(
                domain_only_plan(),
                TokioDriver::new(),
                NativePluginRegistry::new()
                    .with_factory(EmptyFactory)
                    .with_factory(DomainFactory {
                        verifier: issuer.verifier(),
                        now,
                        draft: Rc::new(RefCell::new(None)),
                        mode: KnowledgeMode::Success,
                        require_actor: false,
                    }),
            )
            .await
            .unwrap();

            let response = app
                .invoke::<knowledge_base::KnowledgeBaseListArticles>(
                    "caller",
                    knowledge_base::LIST_ARTICLES_OPERATION,
                    knowledge_base::ListArticlesRequest {
                        cursor: None,
                        limit: 20,
                        organization_id: "org_1".to_owned(),
                    },
                )
                .await
                .unwrap()
                .unwrap();
            assert!(response.articles.is_empty());
            assert_eq!(
                app.shutdown(StdDuration::from_secs(1)).await,
                ShutdownOutcome::Clean
            );
        })
        .await;
}

async fn invoke(app: &lenso_kernel::NativeApp, request: HandleRequest) -> endpoint::HandleResponse {
    app.invoke::<endpoint::EndpointHandle>("caller", endpoint::HANDLE_OPERATION, request)
        .await
        .unwrap()
        .unwrap()
}

async fn start_web_app(
    issuer: ActorAssertionIssuer,
    now: OffsetDateTime,
    draft: Rc<RefCell<Option<Draft>>>,
    mode: KnowledgeMode,
) -> lenso_kernel::NativeApp {
    Kernel::start_native(
        web_plan(),
        TokioDriver::new(),
        NativePluginRegistry::new()
            .with_linked_factories()
            .with_factory(EmptyFactory)
            .with_factory(TestAuthFactory {
                issuer: issuer.clone(),
                now,
            })
            .with_factory(DomainFactory {
                verifier: issuer.verifier(),
                now,
                draft,
                mode,
                require_actor: true,
            }),
    )
    .await
    .unwrap()
}

#[derive(Clone, Copy, Debug)]
struct EmptyFactory;

impl NativePluginFactory for EmptyFactory {
    fn package_id(&self) -> &'static str {
        CALLER_PACKAGE
    }

    fn instantiate(
        &self,
        _: NativePluginFactoryContext<'_>,
    ) -> Result<NativePluginInstance, RuntimeFailure> {
        Ok(NativePluginInstance::default())
    }
}

#[derive(Clone, Debug)]
struct TestAuthFactory {
    issuer: ActorAssertionIssuer,
    now: OffsetDateTime,
}

impl NativePluginFactory for TestAuthFactory {
    fn package_id(&self) -> &'static str {
        AUTH_PACKAGE
    }

    fn instantiate(
        &self,
        _: NativePluginFactoryContext<'_>,
    ) -> Result<NativePluginInstance, RuntimeFailure> {
        Ok(NativePluginInstance::new(vec![Rc::new(AuthEndpoint::new(
            TestAuth {
                issuer: self.issuer.clone(),
                now: self.now,
            },
        ))]))
    }
}

#[derive(Clone, Debug)]
struct TestAuth {
    issuer: ActorAssertionIssuer,
    now: OffsetDateTime,
}

impl AuthProvider for TestAuth {
    fn authenticate(
        &self,
        _context: InvocationContext,
        request: auth::AuthenticateRequest,
    ) -> NativeRequestFuture<Auth> {
        let result = match request.credential {
            Some(credential)
                if credential.scheme == "bearer"
                    && matches!(credential.value.as_str(), "good" | "wrong-kind") =>
            {
                let actor_kind = if credential.value == "good" {
                    "user"
                } else {
                    "service"
                };
                let assertion = self.issuer.issue(
                    "author_1",
                    actor_kind,
                    "test",
                    KNOWLEDGE_OPERATIONS
                        .iter()
                        .map(|operation| audience(knowledge_base::CAPABILITY_ID, operation)),
                    Validity::new(
                        self.now - Duration::seconds(1),
                        self.now + Duration::minutes(1),
                    )
                    .unwrap(),
                    BTreeMap::new(),
                );
                Ok(Ok(authenticated_response(&assertion)))
            }
            _ => Ok(Err(auth::AuthenticateError::Invalid)),
        };
        Box::pin(std::future::ready(result))
    }
}

#[derive(Clone, Debug)]
struct DomainFactory {
    verifier: lenso_auth_sdk::ActorAssertionVerifier,
    now: OffsetDateTime,
    draft: Rc<RefCell<Option<Draft>>>,
    mode: KnowledgeMode,
    require_actor: bool,
}

impl NativePluginFactory for DomainFactory {
    fn package_id(&self) -> &'static str {
        DOMAIN_PACKAGE
    }

    fn instantiate(
        &self,
        _: NativePluginFactoryContext<'_>,
    ) -> Result<NativePluginInstance, RuntimeFailure> {
        Ok(NativePluginInstance::new(vec![Rc::new(
            knowledge_base::KnowledgeBaseEndpoint::new(FakeKnowledgeBase {
                verifier: self.verifier.clone(),
                now: self.now,
                draft: Rc::clone(&self.draft),
                mode: self.mode,
                require_actor: self.require_actor,
            }),
        )]))
    }
}

#[derive(Clone, Debug)]
struct Draft {
    article_id: String,
    organization_id: String,
    slug: String,
    title: String,
    body_markdown: String,
    revision: u64,
    latest_publication_revision: Option<String>,
    latest_published_article_revision: Option<String>,
}

#[derive(Debug)]
struct FakeKnowledgeBase {
    verifier: lenso_auth_sdk::ActorAssertionVerifier,
    now: OffsetDateTime,
    draft: Rc<RefCell<Option<Draft>>>,
    mode: KnowledgeMode,
    require_actor: bool,
}

impl FakeKnowledgeBase {
    fn authorized(&self, context: &InvocationContext, operation: &str) -> bool {
        !self.require_actor
            || self
                .verifier
                .project_context::<AuthorActor>(
                    context,
                    knowledge_base::CAPABILITY_ID,
                    operation,
                    &FixedClock::new(self.now),
                )
                .is_ok()
    }
}

impl knowledge_base::KnowledgeBaseProvider for FakeKnowledgeBase {
    fn create_draft(
        &self,
        context: InvocationContext,
        request: knowledge_base::CreateDraftRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBaseCreateDraft> {
        if !self.authorized(&context, knowledge_base::CREATE_DRAFT_OPERATION) {
            return ready::<knowledge_base::KnowledgeBaseCreateDraft>(Err(
                knowledge_base::CreateDraftError::Unauthenticated,
            ));
        }
        if request.organization_id != "org_1" {
            return ready::<knowledge_base::KnowledgeBaseCreateDraft>(Err(
                knowledge_base::CreateDraftError::Forbidden,
            ));
        }
        if self.draft.borrow().is_some() {
            return ready::<knowledge_base::KnowledgeBaseCreateDraft>(Err(
                knowledge_base::CreateDraftError::SlugConflict,
            ));
        }
        let draft = Draft {
            article_id: "article_1".to_owned(),
            organization_id: request.organization_id,
            slug: request.slug,
            title: request.title,
            body_markdown: request.body_markdown,
            revision: 1,
            latest_publication_revision: None,
            latest_published_article_revision: None,
        };
        let response = create_response(&draft);
        *self.draft.borrow_mut() = Some(draft);
        ready::<knowledge_base::KnowledgeBaseCreateDraft>(Ok(response))
    }

    fn get_draft(
        &self,
        context: InvocationContext,
        request: knowledge_base::GetDraftRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBaseGetDraft> {
        if !self.authorized(&context, knowledge_base::GET_DRAFT_OPERATION) {
            return ready::<knowledge_base::KnowledgeBaseGetDraft>(Err(
                knowledge_base::GetDraftError::Unauthenticated,
            ));
        }
        if request.organization_id != "org_1" {
            return ready::<knowledge_base::KnowledgeBaseGetDraft>(Err(
                knowledge_base::GetDraftError::Forbidden,
            ));
        }
        let Some(draft) = self.draft.borrow().clone() else {
            return ready::<knowledge_base::KnowledgeBaseGetDraft>(Err(
                knowledge_base::GetDraftError::ArticleNotFound,
            ));
        };
        if request.article_id != draft.article_id {
            return ready::<knowledge_base::KnowledgeBaseGetDraft>(Err(
                knowledge_base::GetDraftError::ArticleNotFound,
            ));
        }
        ready::<knowledge_base::KnowledgeBaseGetDraft>(Ok(get_response(&draft)))
    }

    fn get_published_article(
        &self,
        _context: InvocationContext,
        _request: knowledge_base::GetPublishedArticleRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBaseGetPublishedArticle> {
        runtime_unavailable::<knowledge_base::KnowledgeBaseGetPublishedArticle>()
    }

    fn list_articles(
        &self,
        context: InvocationContext,
        request: knowledge_base::ListArticlesRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBaseListArticles> {
        if !self.authorized(&context, knowledge_base::LIST_ARTICLES_OPERATION) {
            return ready::<knowledge_base::KnowledgeBaseListArticles>(Err(
                knowledge_base::ListArticlesError::Unauthenticated,
            ));
        }
        match self.mode {
            KnowledgeMode::Forbidden => {
                return ready::<knowledge_base::KnowledgeBaseListArticles>(Err(
                    knowledge_base::ListArticlesError::Forbidden,
                ));
            }
            KnowledgeMode::Runtime => {
                return runtime_unavailable::<knowledge_base::KnowledgeBaseListArticles>();
            }
            KnowledgeMode::Success => {}
        }
        if request.organization_id != "org_1" || !(1..=100).contains(&request.limit) {
            return ready::<knowledge_base::KnowledgeBaseListArticles>(Err(
                knowledge_base::ListArticlesError::InvalidRequest,
            ));
        }
        let articles = self
            .draft
            .borrow()
            .as_ref()
            .map(summary_response)
            .into_iter()
            .collect();
        ready::<knowledge_base::KnowledgeBaseListArticles>(Ok(
            knowledge_base::ListArticlesResponse {
                articles,
                next_cursor: None,
            },
        ))
    }

    fn publish_article(
        &self,
        context: InvocationContext,
        request: knowledge_base::PublishArticleRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBasePublishArticle> {
        if !self.authorized(&context, knowledge_base::PUBLISH_ARTICLE_OPERATION) {
            return ready::<knowledge_base::KnowledgeBasePublishArticle>(Err(
                knowledge_base::PublishArticleError::Unauthenticated,
            ));
        }
        if request.organization_id != "org_1" {
            return ready::<knowledge_base::KnowledgeBasePublishArticle>(Err(
                knowledge_base::PublishArticleError::Forbidden,
            ));
        }
        let mut stored = self.draft.borrow_mut();
        let Some(draft) = stored.as_mut() else {
            return ready::<knowledge_base::KnowledgeBasePublishArticle>(Err(
                knowledge_base::PublishArticleError::ArticleNotFound,
            ));
        };
        if request.article_id != draft.article_id {
            return ready::<knowledge_base::KnowledgeBasePublishArticle>(Err(
                knowledge_base::PublishArticleError::ArticleNotFound,
            ));
        }
        if request.expected_revision != revision(draft) {
            return ready::<knowledge_base::KnowledgeBasePublishArticle>(Err(
                knowledge_base::PublishArticleError::RevisionConflict,
            ));
        }
        draft.latest_publication_revision = Some("publication-1".to_owned());
        draft.latest_published_article_revision = Some(revision(draft));
        ready::<knowledge_base::KnowledgeBasePublishArticle>(Ok(publish_response(draft)))
    }

    fn search_published_articles(
        &self,
        _context: InvocationContext,
        _request: knowledge_base::SearchPublishedArticlesRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBaseSearchPublishedArticles> {
        runtime_unavailable::<knowledge_base::KnowledgeBaseSearchPublishedArticles>()
    }

    fn update_draft(
        &self,
        context: InvocationContext,
        request: knowledge_base::UpdateDraftRequest,
    ) -> NativeRequestFuture<knowledge_base::KnowledgeBaseUpdateDraft> {
        if !self.authorized(&context, knowledge_base::UPDATE_DRAFT_OPERATION) {
            return ready::<knowledge_base::KnowledgeBaseUpdateDraft>(Err(
                knowledge_base::UpdateDraftError::Unauthenticated,
            ));
        }
        if request.organization_id != "org_1" {
            return ready::<knowledge_base::KnowledgeBaseUpdateDraft>(Err(
                knowledge_base::UpdateDraftError::Forbidden,
            ));
        }
        let mut stored = self.draft.borrow_mut();
        let Some(draft) = stored.as_mut() else {
            return ready::<knowledge_base::KnowledgeBaseUpdateDraft>(Err(
                knowledge_base::UpdateDraftError::ArticleNotFound,
            ));
        };
        if request.article_id != draft.article_id {
            return ready::<knowledge_base::KnowledgeBaseUpdateDraft>(Err(
                knowledge_base::UpdateDraftError::ArticleNotFound,
            ));
        }
        if request.expected_revision != revision(draft) {
            return ready::<knowledge_base::KnowledgeBaseUpdateDraft>(Err(
                knowledge_base::UpdateDraftError::RevisionConflict,
            ));
        }
        if let Some(Some(title)) = request.title {
            draft.title = title;
        }
        if let Some(Some(body_markdown)) = request.body_markdown {
            draft.body_markdown = body_markdown;
        }
        draft.revision += 1;
        ready::<knowledge_base::KnowledgeBaseUpdateDraft>(Ok(update_response(draft)))
    }
}

fn ready<C>(result: Result<C::Response, C::DomainError>) -> NativeRequestFuture<C>
where
    C: lenso_kernel::RequestCapability,
{
    Box::pin(std::future::ready(Ok(result)))
}

fn runtime_unavailable<C>() -> NativeRequestFuture<C>
where
    C: lenso_kernel::RequestCapability,
{
    Box::pin(std::future::ready(Err(RuntimeFailure::Unavailable {
        capability: knowledge_base::CAPABILITY_ID,
    })))
}

#[derive(Debug)]
struct AuthorActor;

impl TypedActor for AuthorActor {
    fn from_assertion(assertion: &ActorAssertion) -> Result<Self, ActorProjectionError> {
        if assertion.actor_kind() != "user" || assertion.subject() != "author_1" {
            return Err(ActorProjectionError::UnexpectedActorKind {
                expected: "user".to_owned(),
                actual: assertion.actor_kind().to_owned(),
            });
        }
        Ok(Self)
    }
}

fn web_plan() -> ResolvedAppPlan {
    let caller = PluginInstancePlan::new("caller", CALLER_PACKAGE).with_requirement(
        CapabilityRequirementPlan::one(endpoint::CAPABILITY_ID, endpoint::DESCRIPTOR_VERSION),
    );
    let web = PluginInstancePlan::new("knowledge-author-web", PACKAGE_ID)
        .with_capability(CapabilityEndpointPlan::new(
            endpoint::CAPABILITY_ID,
            endpoint::DESCRIPTOR_VERSION,
            [endpoint::DESCRIBE_OPERATION, endpoint::HANDLE_OPERATION],
        ))
        .with_requirement(CapabilityRequirementPlan::one(
            auth::CAPABILITY_ID,
            auth::DESCRIPTOR_VERSION,
        ))
        .with_requirement(CapabilityRequirementPlan::one(
            knowledge_base::CAPABILITY_ID,
            knowledge_base::DESCRIPTOR_VERSION,
        ));
    let auth_provider =
        PluginInstancePlan::new("auth", AUTH_PACKAGE).with_capability(CapabilityEndpointPlan::new(
            auth::CAPABILITY_ID,
            auth::DESCRIPTOR_VERSION,
            [auth::AUTHENTICATE_OPERATION],
        ));
    AppComposition::new(
        vec![caller, web, auth_provider, domain_instance()],
        vec![
            CapabilityBinding::new(
                "caller",
                endpoint::CAPABILITY_ID,
                endpoint::DESCRIPTOR_VERSION,
                "knowledge-author-web",
            ),
            CapabilityBinding::new(
                "knowledge-author-web",
                auth::CAPABILITY_ID,
                auth::DESCRIPTOR_VERSION,
                "auth",
            ),
            CapabilityBinding::new(
                "knowledge-author-web",
                knowledge_base::CAPABILITY_ID,
                knowledge_base::DESCRIPTOR_VERSION,
                "domain",
            ),
        ],
    )
    .resolve()
    .unwrap()
}

fn domain_only_plan() -> ResolvedAppPlan {
    let caller = PluginInstancePlan::new("caller", CALLER_PACKAGE).with_requirement(
        CapabilityRequirementPlan::one(
            knowledge_base::CAPABILITY_ID,
            knowledge_base::DESCRIPTOR_VERSION,
        ),
    );
    AppComposition::new(
        vec![caller, domain_instance()],
        vec![CapabilityBinding::new(
            "caller",
            knowledge_base::CAPABILITY_ID,
            knowledge_base::DESCRIPTOR_VERSION,
            "domain",
        )],
    )
    .resolve()
    .unwrap()
}

fn domain_instance() -> PluginInstancePlan {
    PluginInstancePlan::new("domain", DOMAIN_PACKAGE).with_capability(CapabilityEndpointPlan::new(
        knowledge_base::CAPABILITY_ID,
        knowledge_base::DESCRIPTOR_VERSION,
        KNOWLEDGE_OPERATIONS.iter().copied(),
    ))
}

fn list_request(token: &str) -> HandleRequest {
    handle_request(
        "knowledge.author.web.articles.list",
        "GET",
        "/api/knowledge/articles",
        Vec::new(),
        Some("organization_id=org_1&limit=40"),
        None,
        token,
    )
}

fn create_request(token: &str) -> HandleRequest {
    handle_request(
        "knowledge.author.web.articles.create",
        "POST",
        "/api/knowledge/articles",
        Vec::new(),
        None,
        Some(json!({
            "body_markdown": "# Reset your password",
            "idempotency_key": "create-1",
            "organization_id": "org_1",
            "slug": "reset-your-password",
            "title": "Reset your password"
        })),
        token,
    )
}

fn get_request(token: &str) -> HandleRequest {
    handle_request(
        "knowledge.author.web.articles.get",
        "GET",
        "/api/knowledge/articles/article_1",
        vec![("article_id", "article_1")],
        Some("organization_id=org_1"),
        None,
        token,
    )
}

fn update_request(token: &str, expected_revision: &str, idempotency_key: &str) -> HandleRequest {
    handle_request(
        "knowledge.author.web.articles.update",
        "PATCH",
        "/api/knowledge/articles/article_1",
        vec![("article_id", "article_1")],
        None,
        Some(json!({
            "article_id": "article_1",
            "body_markdown": "# Reset a forgotten password",
            "expected_revision": expected_revision,
            "idempotency_key": idempotency_key,
            "organization_id": "org_1",
            "title": "Reset a forgotten password"
        })),
        token,
    )
}

fn publish_request(token: &str, expected_revision: &str) -> HandleRequest {
    handle_request(
        "knowledge.author.web.articles.publish",
        "POST",
        "/api/knowledge/articles/article_1/publish",
        vec![("article_id", "article_1")],
        None,
        Some(json!({
            "article_id": "article_1",
            "expected_revision": expected_revision,
            "idempotency_key": "publish-1",
            "organization_id": "org_1"
        })),
        token,
    )
}

fn handle_request(
    route_id: &str,
    method: &str,
    path: &str,
    path_parameters: Vec<(&str, &str)>,
    query: Option<&str>,
    body: Option<Value>,
    token: &str,
) -> HandleRequest {
    let has_body = body.is_some();
    HandleRequest {
        body: body
            .map_or_else(Vec::new, |value| serde_json::to_vec(&value).unwrap())
            .into(),
        credential: Some(HandleRequestCredential {
            scheme: "bearer".to_owned(),
            value: token.to_owned(),
        }),
        headers: if has_body {
            vec![HandleRequestHeadersItem {
                name: "content-type".to_owned(),
                value: "application/json".to_owned(),
            }]
        } else {
            Vec::new()
        },
        method: method.to_owned(),
        path: path.to_owned(),
        path_parameters: path_parameters
            .into_iter()
            .map(|(name, value)| HandleRequestPathParametersItem {
                name: name.to_owned(),
                value: value.to_owned(),
            })
            .collect(),
        query: query.map(str::to_owned),
        request_id: format!("test-{route_id}"),
        route_id: route_id.to_owned(),
    }
}

fn revision(draft: &Draft) -> String {
    format!("rev-{}", draft.revision)
}

fn create_response(draft: &Draft) -> knowledge_base::CreateDraftResponse {
    fixture(json!({
        "article_id": draft.article_id,
        "body_markdown": draft.body_markdown,
        "created_at": NOW,
        "created_by": "author_1",
        "organization_id": draft.organization_id,
        "revision": revision(draft),
        "slug": draft.slug,
        "title": draft.title,
        "updated_at": NOW
    }))
}

fn update_response(draft: &Draft) -> knowledge_base::UpdateDraftResponse {
    fixture(json!({
        "article_id": draft.article_id,
        "body_markdown": draft.body_markdown,
        "created_at": NOW,
        "created_by": "author_1",
        "organization_id": draft.organization_id,
        "revision": revision(draft),
        "slug": draft.slug,
        "title": draft.title,
        "updated_at": NOW
    }))
}

fn get_response(draft: &Draft) -> knowledge_base::GetDraftResponse {
    fixture(json!({
        "article_id": draft.article_id,
        "body_markdown": draft.body_markdown,
        "created_at": NOW,
        "created_by": "author_1",
        "latest_publication_revision": draft.latest_publication_revision,
        "latest_published_article_revision": draft.latest_published_article_revision,
        "latest_published_at": draft.latest_publication_revision.as_ref().map(|_| NOW),
        "latest_published_by": draft.latest_publication_revision.as_ref().map(|_| "author_1"),
        "organization_id": draft.organization_id,
        "revision": revision(draft),
        "slug": draft.slug,
        "title": draft.title,
        "updated_at": NOW,
        "updated_by": "author_1"
    }))
}

fn summary_response(draft: &Draft) -> knowledge_base::ListArticlesResponseArticlesItem {
    fixture(json!({
        "article_id": draft.article_id,
        "created_at": NOW,
        "created_by": "author_1",
        "latest_publication_revision": draft.latest_publication_revision,
        "latest_published_article_revision": draft.latest_published_article_revision,
        "latest_published_at": draft.latest_publication_revision.as_ref().map(|_| NOW),
        "latest_published_by": draft.latest_publication_revision.as_ref().map(|_| "author_1"),
        "revision": revision(draft),
        "slug": draft.slug,
        "title": draft.title,
        "updated_at": NOW,
        "updated_by": "author_1"
    }))
}

fn publish_response(draft: &Draft) -> knowledge_base::PublishArticleResponse {
    fixture(json!({
        "article_id": draft.article_id,
        "article_revision": revision(draft),
        "body_markdown": draft.body_markdown,
        "organization_id": draft.organization_id,
        "publication_revision": "publication-1",
        "published_at": NOW,
        "published_by": "author_1",
        "slug": draft.slug,
        "title": draft.title
    }))
}

fn fixture<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn body_json(response: &endpoint::HandleResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap()
}

fn body_text(response: &endpoint::HandleResponse) -> String {
    String::from_utf8_lossy(&response.body).into_owned()
}
