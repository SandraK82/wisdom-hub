//! gRPC API implementation using tonic
//!
//! Implements the HubService gRPC interface for high-performance communication.

use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::models::{
    self, CreateAgentRequest as ModelCreateAgentRequest,
    CreateFragmentRequest as ModelCreateFragmentRequest,
    CreateRelationRequest as ModelCreateRelationRequest,
    CreateTagRequest as ModelCreateTagRequest,
    CreateTransformRequest as ModelCreateTransformRequest,
    Address, TagCategory,
};
use crate::proto::hub_service_server::HubService;
use crate::proto::{self as pb};
use crate::services::{
    EntityService, TrustService, TrustConfig,
    DiscoveryService, DiscoveryConfig, FederatedSearchService,
};
use crate::config::HubRole;
use crate::store::EntityStore;

/// gRPC server implementation
pub struct HubServiceImpl {
    service: Arc<EntityService>,
    trust_service: Arc<TrustService>,
    federated_search_service: Arc<FederatedSearchService>,
    #[allow(dead_code)]
    hub_id: String,
}

impl HubServiceImpl {
    /// Create a new gRPC service implementation
    pub fn new(service: Arc<EntityService>, store: Arc<EntityStore>) -> Self {
        let trust_service = Arc::new(TrustService::new(Arc::clone(&store), TrustConfig::default()));

        // Create a minimal discovery config for the federated search service
        let discovery_config = DiscoveryConfig {
            role: HubRole::Primary,
            hub_id: "grpc-hub".to_string(),
            public_url: "http://localhost:8080".to_string(),
            ..Default::default()
        };
        let discovery_service = Arc::new(DiscoveryService::new(discovery_config, Arc::clone(&store)));

        let federated_search_service = Arc::new(FederatedSearchService::new(
            Arc::clone(&service),
            Arc::clone(&discovery_service),
        ));

        Self {
            service,
            trust_service,
            federated_search_service,
            hub_id: "grpc-hub".to_string(),
        }
    }

    /// Create with a custom discovery service
    pub fn with_discovery(
        service: Arc<EntityService>,
        trust_service: Arc<TrustService>,
        discovery_service: Arc<DiscoveryService>,
    ) -> Self {
        let hub_id = discovery_service.hub_id().to_string();
        let federated_search_service = Arc::new(FederatedSearchService::new(
            Arc::clone(&service),
            Arc::clone(&discovery_service),
        ));

        Self {
            service,
            trust_service,
            federated_search_service,
            hub_id,
        }
    }
}

// ============================================================================
// Type Conversions: Internal Models -> Protobuf
// ============================================================================

impl From<models::Agent> for pb::Agent {
    fn from(agent: models::Agent) -> Self {
        pb::Agent {
            uuid: agent.uuid,
            public_key: agent.public_key,
            description: agent.description,
            primary_hub: agent.primary_hub,
            version: agent.version as i32,
            trust: Some(pb::TrustStore {
                trusts: agent.trust.trusts.into_iter().map(|t| pb::Trust {
                    agent_address: t.agent.to_string(),
                    trust_level: t.trust,
                }).collect(),
            }),
            signature: agent.signature,
            created_at: Some(datetime_to_timestamp(agent.created_at)),
            updated_at: Some(datetime_to_timestamp(agent.updated_at)),
        }
    }
}

impl From<models::Fragment> for pb::Fragment {
    fn from(fragment: models::Fragment) -> Self {
        pb::Fragment {
            uuid: fragment.uuid,
            tag_addresses: fragment.tags.into_iter().map(|a| a.to_string()).collect(),
            transform_address: fragment.transform.map(|a| a.to_string()).unwrap_or_default(),
            content: fragment.content,
            content_hash: fragment.content_hash,
            creator_address: fragment.creator.to_string(),
            version: fragment.version as i32,
            when: Some(datetime_to_timestamp(fragment.when)),
            signature: fragment.signature,
            created_at: Some(datetime_to_timestamp(fragment.created_at)),
            updated_at: Some(datetime_to_timestamp(fragment.updated_at)),
        }
    }
}

impl From<models::Relation> for pb::Relation {
    fn from(relation: models::Relation) -> Self {
        pb::Relation {
            uuid: relation.uuid,
            from_address: relation.from.to_string(),
            to_address: relation.to.to_string(),
            relation_type: relation.relation_type.to_string(),
            creator_address: relation.creator.to_string(),
            version: relation.version as i32,
            signature: relation.signature,
            created_at: Some(datetime_to_timestamp(relation.created_at)),
        }
    }
}

impl From<models::Tag> for pb::Tag {
    fn from(tag: models::Tag) -> Self {
        pb::Tag {
            uuid: tag.uuid,
            name: tag.name,
            content: tag.content,
            category: tag.category.to_string(),
            creator_address: tag.creator.to_string(),
            version: tag.version as i32,
            signature: tag.signature,
            created_at: Some(datetime_to_timestamp(tag.created_at)),
        }
    }
}

impl From<models::Transform> for pb::Transform {
    fn from(transform: models::Transform) -> Self {
        pb::Transform {
            uuid: transform.uuid,
            name: transform.name,
            description: transform.description,
            tag_addresses: transform.tags.into_iter().map(|a| a.to_string()).collect(),
            transform_to: transform.transform_to,
            transform_from: transform.transform_from,
            additional_data: transform.additional_data,
            agent_address: transform.agent.to_string(),
            version: transform.version as i32,
            signature: transform.signature,
            created_at: Some(datetime_to_timestamp(transform.created_at)),
        }
    }
}

impl From<models::TrustPath> for pb::TrustPath {
    fn from(path: models::TrustPath) -> Self {
        pb::TrustPath {
            from_address: path.from.to_string(),
            to_address: path.to.to_string(),
            hops: path.hops.into_iter().map(|h| pb::TrustPathHop {
                agent_address: h.agent.to_string(),
                trust_level: h.trust_level,
            }).collect(),
            effective_trust: path.effective_trust,
            depth: path.depth as i32,
        }
    }
}

impl From<models::TrustScore> for pb::TrustScore {
    fn from(score: models::TrustScore) -> Self {
        pb::TrustScore {
            entity_address: score.entity.to_string(),
            viewer_address: score.viewer.to_string(),
            score: score.score,
            path_count: score.path_count as i32,
            best_path: score.best_path.map(Into::into),
        }
    }
}

// ============================================================================
// Type Conversions: Protobuf -> Internal Models
// ============================================================================

fn pb_to_create_agent(req: pb::CreateAgentRequest) -> Result<ModelCreateAgentRequest, Status> {
    Ok(ModelCreateAgentRequest {
        uuid: if req.uuid.is_empty() { None } else { Some(req.uuid) },
        public_key: req.public_key,
        description: if req.description.is_empty() { None } else { Some(req.description) },
        primary_hub: if req.primary_hub.is_empty() { None } else { Some(req.primary_hub) },
        signature: req.signature,
    })
}

fn pb_to_create_fragment(req: pb::CreateFragmentRequest) -> Result<ModelCreateFragmentRequest, Status> {
    let creator = Address::parse(&req.created_by)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid creator address: {}", req.created_by)))?;

    let tags: Option<Vec<Address>> = if req.tag_addresses.is_empty() {
        None
    } else {
        Some(req.tag_addresses
            .iter()
            .map(|s| Address::parse(s).ok_or_else(|| Status::invalid_argument(format!("Invalid tag address: {}", s))))
            .collect::<Result<Vec<_>, _>>()?)
    };

    let transform: Option<Address> = if req.transform_address.is_empty() {
        None
    } else {
        Some(Address::parse(&req.transform_address)
            .ok_or_else(|| Status::invalid_argument(format!("Invalid transform address: {}", req.transform_address)))?)
    };

    Ok(ModelCreateFragmentRequest {
        uuid: if req.uuid.is_empty() { None } else { Some(req.uuid) },
        tags,
        transform,
        content: req.content,
        creator,
        when: None,
        signature: req.signature,
        confidence: None,
        evidence_type: None,
    })
}

fn pb_to_create_relation(req: pb::CreateRelationRequest) -> Result<ModelCreateRelationRequest, Status> {
    let from = Address::parse(&req.from_address)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid from address: {}", req.from_address)))?;
    let to = Address::parse(&req.to_address)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid to address: {}", req.to_address)))?;
    let creator = Address::parse(&req.created_by)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid creator address: {}", req.created_by)))?;

    Ok(ModelCreateRelationRequest {
        uuid: if req.uuid.is_empty() { None } else { Some(req.uuid) },
        from,
        to,
        by: creator.clone(),
        r#type: req.relation_type,
        content: None,
        creator,
        when: None,
        signature: req.signature,
        confidence: None,
    })
}

fn pb_to_create_tag(req: pb::CreateTagRequest) -> Result<ModelCreateTagRequest, Status> {
    let creator = Address::parse(&req.created_by)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid creator address: {}", req.created_by)))?;

    let category: TagCategory = req.category.parse()
        .map_err(|_| Status::invalid_argument(format!("Invalid tag category: {}", req.category)))?;

    Ok(ModelCreateTagRequest {
        uuid: if req.uuid.is_empty() { None } else { Some(req.uuid) },
        name: req.name,
        content: req.content,
        category,
        creator,
        signature: req.signature,
    })
}

fn pb_to_create_transform(req: pb::CreateTransformRequest) -> Result<ModelCreateTransformRequest, Status> {
    let agent = Address::parse(&req.agent_address)
        .ok_or_else(|| Status::invalid_argument(format!("Invalid agent address: {}", req.agent_address)))?;

    let tags: Vec<Address> = req.tag_addresses
        .iter()
        .map(|s| Address::parse(s).ok_or_else(|| Status::invalid_argument(format!("Invalid tag address: {}", s))))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ModelCreateTransformRequest {
        uuid: if req.uuid.is_empty() { None } else { Some(req.uuid) },
        name: req.name,
        description: req.description,
        tags,
        transform_to: req.transform_to,
        transform_from: req.transform_from,
        additional_data: req.additional_data,
        agent,
        signature: req.signature,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn hub_error_to_status(e: models::HubError) -> Status {
    match e {
        models::HubError::NotFound { entity_type, id } => {
            Status::not_found(format!("{} with id {} not found", entity_type, id))
        }
        models::HubError::AlreadyExists { entity_type, id } => {
            Status::already_exists(format!("{} with id {} already exists", entity_type, id))
        }
        models::HubError::InvalidSignature { entity_type } => {
            Status::unauthenticated(format!("Invalid signature for {}", entity_type))
        }
        models::HubError::InvalidContentHash => Status::invalid_argument("Invalid content hash"),
        models::HubError::InvalidPublicKey(msg) => Status::invalid_argument(msg),
        models::HubError::CryptoError(msg) => Status::internal(msg),
        models::HubError::DatabaseError(msg) => Status::internal(msg),
        models::HubError::SerializationError(msg) => Status::internal(msg),
        models::HubError::ConfigError(msg) => Status::internal(msg),
        models::HubError::NetworkError(msg) => Status::unavailable(msg),
        models::HubError::TrustPathNotFound { from, to } => {
            Status::not_found(format!("Trust path not found from {} to {}", from, to))
        }
        models::HubError::FederationError(msg) => Status::unavailable(msg),
        models::HubError::RateLimitExceeded => Status::resource_exhausted("Rate limit exceeded"),
        models::HubError::Unauthorized(msg) => Status::unauthenticated(msg),
        models::HubError::ValidationError(msg) => Status::invalid_argument(msg),
        models::HubError::ResourceLimitExceeded(msg) => Status::resource_exhausted(msg),
        models::HubError::Internal(msg) => Status::internal(msg),
    }
}

// ============================================================================
// Stream Types
// ============================================================================

type FragmentStream = Pin<Box<dyn Stream<Item = Result<pb::Fragment, Status>> + Send>>;

// ============================================================================
// HubService Implementation
// ============================================================================

#[tonic::async_trait]
impl HubService for HubServiceImpl {
    // ========================================================================
    // Agents
    // ========================================================================

    async fn create_agent(
        &self,
        request: Request<pb::CreateAgentRequest>,
    ) -> Result<Response<pb::Agent>, Status> {
        let req = request.into_inner();
        let model_req = pb_to_create_agent(req)?;

        let agent = self.service
            .create_agent(model_req)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(agent.into()))
    }

    async fn get_agent(
        &self,
        request: Request<pb::GetAgentRequest>,
    ) -> Result<Response<pb::Agent>, Status> {
        let uuid = request.into_inner().uuid;

        let agent = self.service
            .get_agent(&uuid)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(agent.into()))
    }

    async fn list_agents(
        &self,
        request: Request<pb::ListAgentsRequest>,
    ) -> Result<Response<pb::ListAgentsResponse>, Status> {
        let req = request.into_inner();
        let cursor = if req.cursor.is_empty() { None } else { Some(req.cursor.as_str()) };
        let limit = req.limit as usize;

        let result = self.service
            .list_agents(cursor, limit)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(pb::ListAgentsResponse {
            agents: result.items.into_iter().map(Into::into).collect(),
            next_cursor: result.next_cursor.unwrap_or_default(),
        }))
    }

    // ========================================================================
    // Fragments
    // ========================================================================

    async fn create_fragment(
        &self,
        request: Request<pb::CreateFragmentRequest>,
    ) -> Result<Response<pb::Fragment>, Status> {
        let model_req = pb_to_create_fragment(request.into_inner())?;

        let fragment = self.service
            .create_fragment(model_req)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(fragment.into()))
    }

    async fn get_fragment(
        &self,
        request: Request<pb::GetFragmentRequest>,
    ) -> Result<Response<pb::Fragment>, Status> {
        let uuid = request.into_inner().uuid;

        let fragment = self.service
            .get_fragment(&uuid)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(fragment.into()))
    }

    type SearchFragmentsStream = FragmentStream;

    async fn search_fragments(
        &self,
        request: Request<pb::SearchFragmentsRequest>,
    ) -> Result<Response<Self::SearchFragmentsStream>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit as usize } else { 20 };

        let results = self.service
            .search_fragments(&req.query, limit)
            .map_err(hub_error_to_status)?;

        let stream = tokio_stream::iter(
            results.into_iter().map(|f| Ok(f.into()))
        );

        Ok(Response::new(Box::pin(stream)))
    }

    // ========================================================================
    // Relations
    // ========================================================================

    async fn create_relation(
        &self,
        request: Request<pb::CreateRelationRequest>,
    ) -> Result<Response<pb::Relation>, Status> {
        let model_req = pb_to_create_relation(request.into_inner())?;

        let relation = self.service
            .create_relation(model_req)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(relation.into()))
    }

    async fn get_relation(
        &self,
        request: Request<pb::GetRelationRequest>,
    ) -> Result<Response<pb::Relation>, Status> {
        let uuid = request.into_inner().uuid;

        let relation = self.service
            .get_relation(&uuid)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(relation.into()))
    }

    // ========================================================================
    // Tags
    // ========================================================================

    async fn create_tag(
        &self,
        request: Request<pb::CreateTagRequest>,
    ) -> Result<Response<pb::Tag>, Status> {
        let model_req = pb_to_create_tag(request.into_inner())?;

        let tag = self.service
            .create_tag(model_req)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(tag.into()))
    }

    async fn get_tag(
        &self,
        request: Request<pb::GetTagRequest>,
    ) -> Result<Response<pb::Tag>, Status> {
        let uuid = request.into_inner().uuid;

        let tag = self.service
            .get_tag(&uuid)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(tag.into()))
    }

    async fn list_tags(
        &self,
        request: Request<pb::ListTagsRequest>,
    ) -> Result<Response<pb::ListTagsResponse>, Status> {
        let req = request.into_inner();
        let cursor = if req.cursor.is_empty() { None } else { Some(req.cursor.as_str()) };
        let limit = req.limit as usize;

        let result = self.service
            .list_tags(cursor, limit)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(pb::ListTagsResponse {
            tags: result.items.into_iter().map(Into::into).collect(),
            next_cursor: result.next_cursor.unwrap_or_default(),
        }))
    }

    // ========================================================================
    // Transforms
    // ========================================================================

    async fn create_transform(
        &self,
        request: Request<pb::CreateTransformRequest>,
    ) -> Result<Response<pb::Transform>, Status> {
        let model_req = pb_to_create_transform(request.into_inner())?;

        let transform = self.service
            .create_transform(model_req)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(transform.into()))
    }

    async fn get_transform(
        &self,
        request: Request<pb::GetTransformRequest>,
    ) -> Result<Response<pb::Transform>, Status> {
        let uuid = request.into_inner().uuid;

        let transform = self.service
            .get_transform(&uuid)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(transform.into()))
    }

    // ========================================================================
    // Trust
    // ========================================================================

    async fn calculate_trust_path(
        &self,
        request: Request<pb::TrustPathRequest>,
    ) -> Result<Response<pb::TrustPath>, Status> {
        let req = request.into_inner();

        let from = Address::parse(&req.from_address)
            .ok_or_else(|| Status::invalid_argument(format!("Invalid from address: {}", req.from_address)))?;
        let to = Address::parse(&req.to_address)
            .ok_or_else(|| Status::invalid_argument(format!("Invalid to address: {}", req.to_address)))?;

        let path = self.trust_service
            .find_best_path(&from, &to)
            .map_err(hub_error_to_status)?;

        match path {
            Some(p) => Ok(Response::new(p.into())),
            None => Err(Status::not_found("No trust path found")),
        }
    }

    async fn get_trust_score(
        &self,
        request: Request<pb::TrustScoreRequest>,
    ) -> Result<Response<pb::TrustScore>, Status> {
        let req = request.into_inner();

        let entity = Address::parse(&req.entity_address)
            .ok_or_else(|| Status::invalid_argument(format!("Invalid entity address: {}", req.entity_address)))?;
        let viewer = Address::parse(&req.viewer_address)
            .ok_or_else(|| Status::invalid_argument(format!("Invalid viewer address: {}", req.viewer_address)))?;

        let score = self.trust_service
            .calculate_trust_score(&entity, &viewer)
            .map_err(hub_error_to_status)?;

        Ok(Response::new(score.into()))
    }

    // ========================================================================
    // Discovery
    // ========================================================================

    async fn register_hub(
        &self,
        _request: Request<pb::HubRegistration>,
    ) -> Result<Response<pb::RegistrationResponse>, Status> {
        Err(Status::unimplemented("Hub registration via gRPC not implemented"))
    }

    async fn heartbeat(
        &self,
        _request: Request<pb::HeartbeatRequest>,
    ) -> Result<Response<pb::HeartbeatResponse>, Status> {
        Err(Status::unimplemented("Heartbeat via gRPC not implemented"))
    }

    async fn get_known_hubs(
        &self,
        _request: Request<()>,
    ) -> Result<Response<pb::HubList>, Status> {
        Err(Status::unimplemented("Hub discovery via gRPC not implemented"))
    }

    // ========================================================================
    // Federated Search
    // ========================================================================

    async fn federated_search(
        &self,
        request: Request<pb::FederatedSearchRequest>,
    ) -> Result<Response<pb::FederatedSearchResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit as usize } else { 20 };
        let min_results = if req.min_results > 0 { Some(req.min_results as usize) } else { None };

        let result = self.federated_search_service
            .search(&req.query, limit, req.federate, min_results)
            .await
            .map_err(hub_error_to_status)?;

        // Convert results to protobuf
        let results: Vec<pb::SearchResult> = result.results
            .into_iter()
            .map(|r| pb::SearchResult {
                fragment: Some(r.fragment.into()),
                source_hub_id: r.source_hub_id,
                relevance_score: r.relevance_score as f32,
            })
            .collect();

        let sources: Vec<pb::SearchSource> = result.sources
            .into_iter()
            .map(|s| pb::SearchSource {
                hub_id: s.hub_id,
                count: s.count as i32,
            })
            .collect();

        Ok(Response::new(pb::FederatedSearchResponse {
            results,
            sources,
            federated: result.federated,
            total: result.total as i32,
        }))
    }
}

/// Create a new gRPC server router
pub fn create_grpc_service(service: Arc<EntityService>, store: Arc<EntityStore>) -> pb::hub_service_server::HubServiceServer<HubServiceImpl> {
    pb::hub_service_server::HubServiceServer::new(HubServiceImpl::new(service, store))
}
