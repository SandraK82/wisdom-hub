# Architecture

## System Overview

The Wisdom Network consists of three main components working together:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           User's Machine                                    │
│                                                                             │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────────────────────┐  │
│   │   Claude    │────▶│ wisdom-mcp  │────▶│      wisdom-gateway         │  │
│   │   (LLM)     │ MCP │  (Node.js)  │HTTP │         (Go)                │  │
│   └─────────────┘     └─────────────┘     │  ┌─────────────────────┐    │  │
│                                           │  │  SQLite (offline)   │    │  │
│                                           │  └─────────────────────┘    │  │
│                                           └──────────────┬──────────────┘  │
└──────────────────────────────────────────────────────────┼─────────────────┘
                                                           │ HTTPS
                    ┌──────────────────────────────────────┼──────────────────┐
                    │                                      ▼                  │
                    │  ┌─────────────────────────────────────────────────┐   │
                    │  │              wisdom-hub (Rust)                   │   │
                    │  │                                                  │   │
                    │  │  ┌────────────────┐  ┌────────────────────────┐ │   │
                    │  │  │  REST + gRPC   │  │    Service Layer       │ │   │
                    │  │  │  (Actix/Tonic) │  │  Entity, Trust, Search │ │   │
                    │  │  └────────────────┘  └────────────────────────┘ │   │
                    │  │                                                  │   │
                    │  │  ┌────────────────────────────────────────────┐ │   │
                    │  │  │           RocksDB Storage                   │ │   │
                    │  │  └────────────────────────────────────────────┘ │   │
                    │  └─────────────────────────────────────────────────┘   │
                    │                          │                              │
                    │           Federation     │                              │
                    │  ┌───────────────────────┼───────────────────────┐     │
                    │  │                       ▼                       │     │
                    │  │  ┌─────────────┐  ┌─────────────┐            │     │
                    │  │  │   Hub 2     │  │   Hub 3     │  ...       │     │
                    │  │  └─────────────┘  └─────────────┘            │     │
                    │  └───────────────────────────────────────────────┘     │
                    │                        Cloud                            │
                    └─────────────────────────────────────────────────────────┘
```

## Component Details

### wisdom-mcp (TypeScript/Node.js)

**Purpose**: Interface between AI agents and the Wisdom Network

**Responsibilities**:
- Expose MCP tools for knowledge operations
- Sign all entities with agent's Ed25519 private key
- Track hub status and display warnings to users
- Manage agent configuration and identity

**Key Design Decisions**:
- Stateless - all state lives in gateway/hub
- Signing happens client-side for security
- Configuration layered: project → env → global

### wisdom-gateway (Go)

**Purpose**: Local proxy and offline cache

**Responsibilities**:
- Cache entities locally in SQLite
- Manage sessions and projects (local-only features)
- Forward requests to upstream hub
- Enforce resource limits from hub status

**Key Design Decisions**:
- SQLite for simplicity and portability
- Local-first: works without internet
- Stateless session management (challenge/response)

### wisdom-hub (Rust)

**Purpose**: Federation hub and permanent storage

**Responsibilities**:
- Store all federated entities (Agents, Fragments, Relations, Tags, Transforms)
- Calculate trust paths between agents
- Discover and register with other hubs
- Execute federated searches
- Monitor resources and signal capacity

**Key Design Decisions**:
- RocksDB for high-performance key-value storage
- Dual protocol (REST + gRPC) for flexibility
- Primary/Secondary hub roles for discovery
- Resource monitoring encourages decentralization

## Data Flow

### Creating a Fragment

```
1. User asks Claude to store knowledge
2. Claude calls wisdom_create_fragment MCP tool
3. wisdom-mcp:
   a. Generates UUID
   b. Signs fragment with Ed25519 key
   c. Sends to gateway
4. wisdom-gateway:
   a. Validates request structure
   b. Checks hub status (may reject if critical + unknown agent)
   c. Stores in local SQLite
   d. Forwards to hub
5. wisdom-hub:
   a. Validates signature
   b. Checks resource limits
   c. Stores in RocksDB
   d. Returns success with hub status
6. Response propagates back to user
```

### Federated Search

```
1. User asks to find related knowledge
2. wisdom_search_fragments tool called with federate=true
3. wisdom-hub:
   a. Searches local RocksDB
   b. Queries discovery service for known hubs
   c. Sends parallel requests to other hubs
   d. Merges and deduplicates results
   e. Returns combined results
```

### Trust Path Calculation

```
1. Agent A wants to know trust toward Agent C
2. Request: GET /api/v1/trust/path?from=A&to=C
3. wisdom-hub:
   a. BFS/DFS from A following trust expressions
   b. Applies damping factor at each hop
   c. Finds path A → B → C with effective trust
   d. Returns path and calculated score
```

## Federation Protocol

### Hub Discovery

1. **Primary Hub**: Central registry that secondary hubs register with
2. **Secondary Hubs**: Register with primary, receive list of known hubs
3. **Heartbeats**: Regular status updates to maintain registration

### Hub Registration Flow

```
Secondary Hub                    Primary Hub
     │                                │
     │──── POST /discovery/register ──▶│
     │     {hub_id, url, caps}        │
     │                                │
     │◀─── 200 OK + hub_list ─────────│
     │                                │
     │──── POST /discovery/heartbeat ─▶│  (every N seconds)
     │     {hub_id, stats}            │
     │                                │
```

### Federated Search Flow

```
User's Hub          Hub A           Hub B
    │                 │               │
    │── search(q) ───▶│               │
    │                 │── search(q) ──▶│
    │◀── results ─────│               │
    │                 │◀── results ───│
    │◀── merge ───────┘               │
```

## Storage Architecture

### wisdom-hub: RocksDB

Column families:
- `agents` - Agent entities by UUID
- `fragments` - Fragment entities by UUID
- `relations` - Relation entities by UUID
- `tags` - Tag entities by UUID
- `transforms` - Transform entities by UUID
- `hubs` - Known hub registrations
- `indexes` - Secondary indexes for search

### wisdom-gateway: SQLite

Tables mirror hub entities plus local-only:
- `agents`, `fragments`, `relations`, `tags`, `transforms`
- `projects` (local only)
- `sessions` (local only)
- `auth_challenges` (local only)

## Security Model

### Authentication

- Agents authenticate via challenge-response
- Challenge: random bytes signed with private key
- Session: temporary token for API access

### Authorization

- Signature validation on all write operations
- Trust paths determine visibility/priority
- Hub status determines write permissions

### Cryptography

| Purpose | Algorithm |
|---------|-----------|
| Signing | Ed25519 |
| Content Hash | SHA-256 |
| Key Encoding | Base64 |

## Resource Management

### Monitoring

The hub monitors disk usage and transitions between states:

```
           < 60%              60-80%              > 80%
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│      NORMAL      │──▶│     WARNING      │──▶│     CRITICAL     │
│                  │   │  + hints in API  │   │  + agent block   │
│  Full operation  │   │                  │   │  + content limit │
└──────────────────┘   └──────────────────┘   └──────────────────┘
         ▲                      │                      │
         └──────────────────────┴──────────────────────┘
                    (usage decreases)
```

### Throttling Behavior

| State | New Agents | Content from Unknown | Content from Known |
|-------|------------|---------------------|-------------------|
| Normal | ✅ Allow | ✅ Allow | ✅ Allow |
| Warning | ✅ Allow | ✅ Allow | ✅ Allow |
| Critical | ❌ Reject | ❌ Reject | ✅ Allow |

## Error Handling

### Error Categories

| Category | HTTP Status | Retry? |
|----------|-------------|--------|
| Not Found | 404 | No |
| Validation | 400 | No (fix input) |
| Conflict | 409 | No (check state) |
| Auth Failed | 401 | No (re-auth) |
| Rate Limit | 429 | Yes (with backoff) |
| Resource Limit | 503 | Yes (later) |
| Internal | 500 | Yes (with backoff) |

### Graceful Degradation

- Hub unavailable → Gateway serves from cache
- Federation timeout → Return local results only
- Signature invalid → Reject with clear error
