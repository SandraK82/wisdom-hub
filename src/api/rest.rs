//! REST API endpoints using Actix-Web

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::responses::{ApiResponse, PaginatedResponse};
use crate::models::{
    CreateAgentRequest, CreateFragmentRequest, CreateRelationRequest,
    CreateTagRequest, CreateTransformRequest, Address,
};
use crate::resources::{ResourceMonitor, ResourceLevel};
use crate::services::{
    EntityService, TrustService, TrustConfig,
    DiscoveryService, DiscoveryConfig, RegisterHubRequest, HeartbeatRequest as ServiceHeartbeatRequest,
    FederatedSearchService,
};
use crate::store::EntityStore;

use super::health::configure_health_routes;

/// Shared application state
#[derive(Clone)]
pub struct ApiState {
    pub service: Arc<EntityService>,
    pub trust_service: Arc<TrustService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub federated_search_service: Arc<FederatedSearchService>,
    pub resource_monitor: Arc<ResourceMonitor>,
}

impl ApiState {
    /// Create a new API state with all services
    pub fn new(store: Arc<EntityStore>, discovery_config: DiscoveryConfig, resource_monitor: Arc<ResourceMonitor>) -> Self {
        let service = Arc::new(EntityService::new(Arc::clone(&store)));
        let trust_service = Arc::new(TrustService::new(Arc::clone(&store), TrustConfig::default()));
        let discovery_service = Arc::new(DiscoveryService::new(discovery_config, Arc::clone(&store)));

        let federated_search_service = Arc::new(FederatedSearchService::new(
            Arc::clone(&service),
            Arc::clone(&discovery_service),
        ));

        Self {
            service,
            trust_service,
            discovery_service,
            federated_search_service,
            resource_monitor,
        }
    }
}

/// Configure all REST API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Health endpoints at root
    configure_health_routes(cfg);

    // API v1 routes
    cfg.service(
        web::scope("/api/v1")
            .configure(configure_v1_routes)
    );
}

/// Configure API v1 routes
fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Agent routes
        .service(
            web::scope("/agents")
                .route("", web::get().to(list_agents))
                .route("", web::post().to(create_agent))
                .route("/{uuid}", web::get().to(get_agent))
                .route("/{uuid}", web::delete().to(delete_agent))
        )
        // Fragment routes
        .service(
            web::scope("/fragments")
                .route("", web::get().to(list_fragments))
                .route("", web::post().to(create_fragment))
                .route("/{uuid}", web::get().to(get_fragment))
                .route("/{uuid}", web::delete().to(delete_fragment))
                .route("/search", web::get().to(search_fragments))
        )
        // Relation routes
        .service(
            web::scope("/relations")
                .route("", web::get().to(list_relations))
                .route("", web::post().to(create_relation))
                .route("/{uuid}", web::get().to(get_relation))
        )
        // Tag routes
        .service(
            web::scope("/tags")
                .route("", web::get().to(list_tags))
                .route("", web::post().to(create_tag))
                .route("/{uuid}", web::get().to(get_tag))
        )
        // Transform routes
        .service(
            web::scope("/transforms")
                .route("", web::get().to(list_transforms))
                .route("", web::post().to(create_transform))
                .route("/{uuid}", web::get().to(get_transform))
        )
        // Trust routes (trust is embedded in Agent, no separate TrustRelation)
        .service(
            web::scope("/trust")
                .route("/path", web::get().to(get_trust_path))
                .route("/score", web::get().to(get_trust_score))
        )
        // Sync routes
        .service(
            web::scope("/sync")
                .route("/changes", web::get().to(pull_changes))
        )
        // Discovery routes
        .service(
            web::scope("/discovery")
                .route("/hubs", web::get().to(get_known_hubs))
                .route("/register", web::post().to(register_hub))
                .route("/heartbeat", web::post().to(heartbeat))
        )
        // Search routes
        .service(
            web::scope("/search")
                .route("", web::get().to(federated_search))
        );
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

// ============================================================================
// Agent Handlers
// ============================================================================

async fn list_agents(
    state: web::Data<ApiState>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = state.service
        .list_agents(query.cursor.as_deref(), query.limit)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    let total = result.items.len();
    Ok(HttpResponse::Ok().json(PaginatedResponse::new(
        result.items,
        total,
        result.next_cursor,
    )))
}

async fn create_agent(
    state: web::Data<ApiState>,
    body: web::Json<CreateAgentRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check resource limits
    let status = state.resource_monitor.get_status();
    let hub_status = state.resource_monitor.get_hub_status_summary();

    if !state.resource_monitor.check_can_accept_agent(&status) {
        return Ok(HttpResponse::ServiceUnavailable().json(
            ApiResponse::<()>::error_with_status(
                "Hub at capacity. New agents not accepted.",
                hub_status,
            )
        ));
    }

    let agent = state.service
        .create_agent(body.into_inner())
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Created().json(ApiResponse::success_with_status(agent, hub_status)))
}

async fn get_agent(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    let agent = state.service
        .get_agent(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(agent)))
}

async fn delete_agent(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    state.service
        .delete_agent(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::NoContent().finish())
}

// ============================================================================
// Fragment Handlers
// ============================================================================

async fn list_fragments(
    state: web::Data<ApiState>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = state.service
        .list_fragments(query.cursor.as_deref(), query.limit)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    let total = result.items.len();
    Ok(HttpResponse::Ok().json(PaginatedResponse::new(
        result.items,
        total,
        result.next_cursor,
    )))
}

async fn create_fragment(
    state: web::Data<ApiState>,
    body: web::Json<CreateFragmentRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check resource limits
    let status = state.resource_monitor.get_status();
    let hub_status = state.resource_monitor.get_hub_status_summary();

    // At critical level, only known agents can create content
    if status.level == ResourceLevel::Critical {
        // Check if the creator agent is known
        let agent_uuid = &body.creator.entity;
        let agent_known = state.service.get_agent(agent_uuid).is_ok();

        if !state.resource_monitor.check_can_accept_content(&status, agent_known) {
            return Ok(HttpResponse::ServiceUnavailable().json(
                ApiResponse::<()>::error_with_status(
                    "Hub at capacity. Unknown agents cannot create content.",
                    hub_status,
                )
            ));
        }
    }

    let fragment = state.service
        .create_fragment(body.into_inner())
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Created().json(ApiResponse::success_with_status(fragment, hub_status)))
}

async fn get_fragment(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    let fragment = state.service
        .get_fragment(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(fragment)))
}

async fn delete_fragment(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    state.service
        .delete_fragment(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::NoContent().finish())
}

async fn search_fragments(
    state: web::Data<ApiState>,
    query: web::Query<SearchQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let results = state.service
        .search_fragments(&query.q, query.limit)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(PaginatedResponse::new(
        results.clone(),
        results.len(),
        None,
    )))
}

// ============================================================================
// Relation Handlers
// ============================================================================

async fn list_relations(
    state: web::Data<ApiState>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = state.service
        .list_relations(query.cursor.as_deref(), query.limit)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    let total = result.items.len();
    Ok(HttpResponse::Ok().json(PaginatedResponse::new(
        result.items,
        total,
        result.next_cursor,
    )))
}

async fn create_relation(
    state: web::Data<ApiState>,
    body: web::Json<CreateRelationRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let relation = state.service
        .create_relation(body.into_inner())
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Created().json(ApiResponse::success(relation)))
}

async fn get_relation(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    let relation = state.service
        .get_relation(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(relation)))
}

// ============================================================================
// Tag Handlers
// ============================================================================

async fn list_tags(
    state: web::Data<ApiState>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = state.service
        .list_tags(query.cursor.as_deref(), query.limit)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    let total = result.items.len();
    Ok(HttpResponse::Ok().json(PaginatedResponse::new(
        result.items,
        total,
        result.next_cursor,
    )))
}

async fn create_tag(
    state: web::Data<ApiState>,
    body: web::Json<CreateTagRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let tag = state.service
        .create_tag(body.into_inner())
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Created().json(ApiResponse::success(tag)))
}

async fn get_tag(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    let tag = state.service
        .get_tag(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(tag)))
}

// ============================================================================
// Transform Handlers
// ============================================================================

async fn list_transforms(
    state: web::Data<ApiState>,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = state.service
        .list_transforms(query.cursor.as_deref(), query.limit)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    let total = result.items.len();
    Ok(HttpResponse::Ok().json(PaginatedResponse::new(
        result.items,
        total,
        result.next_cursor,
    )))
}

async fn create_transform(
    state: web::Data<ApiState>,
    body: web::Json<CreateTransformRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let transform = state.service
        .create_transform(body.into_inner())
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Created().json(ApiResponse::success(transform)))
}

async fn get_transform(
    state: web::Data<ApiState>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let uuid = path.into_inner();
    let transform = state.service
        .get_transform(&uuid)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(transform)))
}

// ============================================================================
// Trust Handlers (Trust is embedded in Agent, no separate TrustRelation)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TrustPathQuery {
    /// Address string: "server:port/DOMAIN/entity-uuid"
    pub from: String,
    /// Address string: "server:port/DOMAIN/entity-uuid"
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct TrustScoreQuery {
    /// Address of the entity to score
    pub entity: String,
    /// Address of the viewer (perspective)
    pub viewer: String,
}

#[derive(Debug, Serialize)]
pub struct TrustPathResponse {
    pub found: bool,
    pub path: Option<crate::models::TrustPath>,
}

async fn get_trust_path(
    state: web::Data<ApiState>,
    query: web::Query<TrustPathQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    // Parse addresses
    let from = Address::parse(&query.from)
        .ok_or_else(|| actix_web::error::InternalError::from_response(
            format!("Invalid 'from' address: {}", query.from),
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid 'from' address"))
        ))?;

    let to = Address::parse(&query.to)
        .ok_or_else(|| actix_web::error::InternalError::from_response(
            format!("Invalid 'to' address: {}", query.to),
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid 'to' address"))
        ))?;

    let path = state.trust_service
        .find_best_path(&from, &to)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    let response = TrustPathResponse {
        found: path.is_some(),
        path,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

async fn get_trust_score(
    state: web::Data<ApiState>,
    query: web::Query<TrustScoreQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    // Parse addresses
    let entity = Address::parse(&query.entity)
        .ok_or_else(|| actix_web::error::InternalError::from_response(
            format!("Invalid 'entity' address: {}", query.entity),
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid 'entity' address"))
        ))?;

    let viewer = Address::parse(&query.viewer)
        .ok_or_else(|| actix_web::error::InternalError::from_response(
            format!("Invalid 'viewer' address: {}", query.viewer),
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid 'viewer' address"))
        ))?;

    let score = state.trust_service
        .calculate_trust_score(&entity, &viewer)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(score)))
}

// ============================================================================
// Sync Handlers (Not needed - Gateways use Push + Search)
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SyncQuery {
    pub since: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

async fn pull_changes(
    _query: web::Query<SyncQuery>,
) -> HttpResponse {
    // Sync is not needed - Gateways push to Hub and use federated search
    HttpResponse::Gone().json(ApiResponse::<()>::error(
        "Sync endpoint removed. Use POST to push entities and GET /search for federated search."
    ))
}

// ============================================================================
// Discovery Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ApiRegisterHubRequest {
    pub hub_id: String,
    pub public_url: String,
    pub capabilities: Vec<String>,
    pub version: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiHeartbeatRequest {
    pub hub_id: String,
    pub status: String,
    pub stats: crate::discovery::HubStats,
}

async fn get_known_hubs(
    state: web::Data<ApiState>,
) -> Result<HttpResponse, actix_web::Error> {
    let hub_list = state.discovery_service
        .get_known_hubs()
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(hub_list)))
}

async fn register_hub(
    state: web::Data<ApiState>,
    body: web::Json<ApiRegisterHubRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let req = body.into_inner();

    let service_req = RegisterHubRequest {
        hub_id: req.hub_id,
        public_url: req.public_url,
        capabilities: req.capabilities,
        version: req.version,
        public_key: req.public_key,
    };

    let response = state.discovery_service
        .register_hub(service_req)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

async fn heartbeat(
    state: web::Data<ApiState>,
    body: web::Json<ApiHeartbeatRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let req = body.into_inner();

    let service_req = ServiceHeartbeatRequest {
        hub_id: req.hub_id,
        status: req.status,
        stats: req.stats,
    };

    let response = state.discovery_service
        .process_heartbeat(service_req)
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ============================================================================
// Federated Search (Placeholder - Phase 5b)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FederatedSearchQuery {
    pub q: String,
    #[serde(default)]
    pub federate: bool,
    pub min_results: Option<usize>,
    pub limit: Option<usize>,
}

async fn federated_search(
    state: web::Data<ApiState>,
    query: web::Query<FederatedSearchQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let limit = query.limit.unwrap_or(20).min(100);
    let min_results = query.min_results;

    let response = state.federated_search_service
        .search(&query.q, limit, query.federate, min_results)
        .await
        .map_err(|e| actix_web::error::InternalError::from_response(
            e.to_string(),
            HttpResponse::from(e)
        ))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}
