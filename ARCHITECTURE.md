# Architecture Documentation

## System Overview

The Multimodal Agentic AI Knowledge Workflow is a distributed microservices architecture designed for production-grade document intelligence with human-in-the-loop oversight.

## Architecture Diagram

```
┌─────────────────┐
│     User        │
└────────┬────────┘
         │
         ↓
┌─────────────────────────────────────────────────────────┐
│               Rust Warp Orchestrator                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │  Auth/   │  │  Rate    │  │  API     │  │Metrics │ │
│  │  RBAC    │→ │ Limiting │→ │ Gateway  │← │OpenTel │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
│                       ↕                                  │
│              ┌────────────────┐                         │
│              │ Agent Workers  │                         │
│              │ (Async Tasks)  │                         │
│              └────────────────┘                         │
└────────────────────┬────────────────────────────────────┘
                     ↓
          ┌──────────────────────┐
          │   Redis (Pub/Sub)    │
          │ - Agent messaging    │
          │ - Task queue         │
          │ - State cache        │
          └──────────────────────┘
                     ↓
    ┌────────────────┴────────────────┐
    │        Agent Orchestration      │
    │  ┌─────────┐  ┌──────────┐     │
    │  │ Planner │→ │Retriever │     │
    │  └─────────┘  └────┬─────┘     │
    │                    ↓            │
    │  ┌─────────────┐  ┌──────────┐ │
    │  │ Summarizer  │← │ Decision │ │
    │  └─────────────┘  └────┬─────┘ │
    │                        ↓        │
    │                  ┌──────────┐   │
    │                  │  Action  │   │
    │                  └──────────┘   │
    └─────────────────────────────────┘
             │              │
             ↓              ↓
┌────────────────┐  ┌──────────────────┐
│   PostgreSQL   │  │  Python Services │
│ - Documents    │  │ ┌──────────────┐ │
│ - Passages     │  │ │ Ingestion    │ │
│ - Requests     │  │ │ (Extract)    │ │
│ - Tasks        │  │ └──────────────┘ │
│ - Actions      │  │ ┌──────────────┐ │
│ - Audit Logs   │  │ │ Embedding    │ │
│   (Immutable)  │  │ │ (Vectors)    │ │
└────────────────┘  │ └──────────────┘ │
                    │ ┌──────────────┐ │
                    │ │ Vector DB    │ │
                    │ │ (FAISS)      │ │
                    │ └──────────────┘ │
                    └──────────────────┘
```

## Component Details

### 1. Rust Warp Orchestrator

**Responsibilities:**
- HTTP/WebSocket API gateway
- Authentication & authorization
- Agent coordination
- Database transactions
- Metrics & observability

**Technology:**
- Warp (async web framework)
- Tokio (async runtime)
- SQLx (type-safe SQL)
- Redis (pub/sub)
- OpenTelemetry/Prometheus

**Key Modules:**
- `api/`: REST/WebSocket endpoints
- `agents/`: Agent implementations
- `middleware/`: Auth, rate limiting, CORS
- `models.rs`: Type definitions
- `db.rs`: Database pool & queries

### 2. Document Ingestion Service

**Responsibilities:**
- File upload & validation
- Text extraction (PDF, DOCX, PPTX, CSV)
- Passage chunking with overlap
- Metadata extraction

**Technology:**
- FastAPI
- pdfplumber, python-docx, python-pptx
- pandas (CSV processing)

**Endpoints:**
- `POST /extract`: Extract text & passages from document
- `GET /health`: Health check

### 3. Embedding Service

**Responsibilities:**
- Generate vector embeddings
- Batch processing
- Model management

**Technology:**
- FastAPI
- sentence-transformers (all-MiniLM-L6-v2)
- 384-dimensional vectors

**Endpoints:**
- `POST /embed`: Generate embeddings for texts
- `GET /model-info`: Model metadata

### 4. Vector Database Service

**Responsibilities:**
- Hybrid search (vector + lexical)
- Index management
- Similarity scoring

**Technology:**
- FastAPI
- FAISS (in-memory, cosine similarity)
- Simple BM25 approximation

**Endpoints:**
- `POST /index/add`: Add vectors to index
- `POST /search/hybrid`: Hybrid search
- `GET /index/stats`: Index statistics

### 5. Multi-Agent System

#### Planner Agent
- **Input:** User query
- **Output:** Structured plan (steps + actions)
- **LLM:** OpenAI GPT-4 (configurable)
- **Prompt:** "Decompose this query into retrieval and summarization steps"

#### Retriever Agent
- **Input:** Query text
- **Output:** Top-k passages with scores
- **Process:**
  1. Generate query embedding
  2. Hybrid search (vector + lexical)
  3. Re-rank with reciprocal rank fusion (RRF)
  4. Return top-k with metadata

#### Summarizer Agent
- **Input:** Query + retrieved passages
- **Output:** Summary with citations
- **LLM:** OpenAI GPT-4 (configurable)
- **Prompt:** "Summarize based ONLY on these passages. Cite every claim as [source:doc_id:page:passage_id]"
- **Validation:** Cross-check citations against retrieved docs

#### Decision Agent
- **Input:** Summary + query
- **Output:** Required actions (JIRA, Slack, Email)
- **Logic:** Rule-based (configurable)
- **Priority:** Score-based ranking

#### Action Agent
- **Input:** Action specification
- **Output:** Execution result
- **Services:** JIRA, Slack, Email (OAuth2)
- **Approval:** All actions queued to `pending_actions` table

### 6. Data Layer

#### PostgreSQL Schema

```sql
documents (id, filename, content_type, s3_key, user_id, metadata)
passages (id, doc_id, passage_index, text, char_start, char_end, page_num)
embeddings_meta (id, passage_id, embedding_model, vector_db_id)
requests (id, user_id, query, created_at, completed_at, status)
tasks (id, request_id, agent_type, input, output, status)
pending_actions (id, request_id, action_type, target_service, payload, status)
audit_logs (id, request_id, task_id, event_type, actor, timestamp, details)
  └─> IMMUTABLE (triggers prevent UPDATE/DELETE)
```

#### Redis Usage

- **Pub/Sub Channels:**
  - `agent:planner:request`
  - `agent:retriever:request`
  - `agent:summarizer:request`
  - `agent:decision:request`
  - `agent:action:request`

- **Task Queue:**
  - Background job processing
  - Retry logic

- **State Cache:**
  - Agent context (TTL: 1 hour)
  - Request state

## Data Flow: End-to-End Query

1. **User submits query** → `POST /api/v1/query`
2. **Orchestrator** creates `Request` record
3. **Planner Agent** decomposes query into steps
4. **For each step:**
   - **Retriever Agent**: Hybrid search
   - **Summarizer Agent**: RAG + citation validation
   - **Decision Agent**: Determine required actions
5. **Action Agent** queues actions to `pending_actions`
6. **Orchestrator** returns summary + pending actions
7. **User approves/rejects** → `POST /api/v1/approve`
8. **Action Agent** executes approved actions
9. **Audit Log** records full provenance

## Security Model

### Authentication
- JWT-based (OAuth2 scaffolding)
- Per-request validation
- Refresh token rotation

### Authorization (RBAC)
- User roles: `admin`, `user`, `viewer`
- Action permissions: `read`, `write`, `approve`, `admin`
- Resource-level ACLs

### Audit Trail
- **Immutable logs** (PostgreSQL triggers)
- **Provenance:** Full prompt + response + retrieved docs
- **Compliance:** GDPR, SOC2 ready

### Secret Management
- Environment variables (dev)
- HashiCorp Vault (production)
- Rotation policies

## Scalability & Performance

### Horizontal Scaling
- Orchestrator: Stateless, scale behind load balancer
- Python services: Containerized, scale independently
- Redis: Sentinel for HA
- PostgreSQL: Read replicas for analytics

### Caching Strategy
- Redis: Agent state, query results (TTL: 5 min)
- Vector DB: In-memory FAISS (production: Milvus/Pinecone)
- HTTP caching: CDN for static assets

### Optimization
- Batch embedding generation
- Connection pooling (PgBouncer)
- Async I/O throughout
- Lazy model loading

## Observability

### Metrics (Prometheus)
- Request latency (p50, p95, p99)
- Agent execution time
- LLM token usage
- Vector search latency
- Error rates

### Logging (Structured JSON)
- Request ID tracing
- Correlation across services
- Error context

### Tracing (OpenTelemetry + Jaeger)
- Distributed traces
- Service dependencies
- Latency breakdown

## Deployment

### Development
- Docker Compose
- Hot reloading (services)
- Local PostgreSQL/Redis

### Production
- Kubernetes (Helm charts)
- Managed PostgreSQL (RDS/CloudSQL)
- Redis Cluster
- Production vector DB (Milvus/Pinecone)
- S3 for document storage
- Ingress with TLS

## Future Enhancements

### Phase 2: Multimodal
- OCR integration (PaddleOCR/TrOCR)
- Table extraction (camelot-py)
- Audio transcription (Whisper)
- Vision-LLM (GPT-4V)

### Phase 3: Auto-Actions
- External connectors (JIRA, Slack, GDrive)
- Sandboxed code execution
- Advanced model routing
- Multi-level approvals

### Phase 4: Enterprise
- SSO integration
- Advanced RBAC
- Compliance certifications
- Multi-tenancy
