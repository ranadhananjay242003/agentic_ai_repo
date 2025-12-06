# Multimodal Agentic AI Knowledge Workflow System

A production-grade, multimodal agentic AI system that ingests enterprise documents, performs hybrid vector+lexical search, orchestrates multiple AI agents, enforces human-in-the-loop workflows, and maintains full audit trails with mandatory citations.

## 🏗️ Architecture

### Core Components

1. **Rust Warp Orchestrator** (Port 8080)
   - High-performance async API gateway
   - Agent coordination via Redis pub/sub
   - RBAC, OAuth2, rate limiting
   - OpenTelemetry/Prometheus metrics
   - WebSocket streaming for real-time query progress

2. **Document Ingestion Service** (Port 8001) - Python/FastAPI
   - Extracts text from PDF, DOCX, PPTX, CSV
   - Passage-level chunking with overlap
   - Metadata extraction
   - Future: OCR (PaddleOCR/TrOCR), audio transcription (Whisper)

3. **Embedding Service** (Port 8002) - Python/FastAPI
   - sentence-transformers (all-MiniLM-L6-v2)
   - Batch embedding generation
   - 384-dimensional vectors

4. **Vector Database Service** (Port 8003) - Python/FastAPI + FAISS
   - Hybrid search (vector + BM25 lexical)
   - Cosine similarity with inner product
   - Production-ready for Pinecone/Milvus/Weaviate migration

5. **Multi-Agent System**
   - **Planner Agent**: Decomposes queries into steps
   - **Retriever Agent**: Top-k hybrid search with re-ranking
   - **Summarizer Agent**: RAG with mandatory citations
   - **Decision Agent**: Business rules, action prioritization
   - **Action Agent**: External integrations (JIRA, Slack, Email)

6. **Data Layer**
   - **Postgres 15**: Metadata, audit logs (immutable), requests, tasks, actions
   - **Redis 7**: Pub/sub, task queue, agent state
   - **S3-compatible storage**: Original documents (planned)

### Key Features

- ✅ **Evidence-first answers** with citation validation
- ✅ **Human-in-the-loop** approval workflow for all actions
- ✅ **Immutable audit trail** for full provenance
- ✅ **Hybrid search** (vector + lexical)
- ✅ **Multimodal support** (text MVP, images/audio planned)
- ✅ **Pluggable LLM backends** (OpenAI, self-hosted)
- ✅ **Production-ready** observability (metrics, tracing, logging)

## 📋 Prerequisites

### Required

- **Rust** (1.70+): [Install rustup](https://rustup.rs/)
- **Docker Desktop** for Windows: [Download](https://www.docker.com/products/docker-desktop/)
- **Node.js** (20+): Already installed ✓
- **Git**: For version control

### Optional

- **Python 3.11**: For local development without Docker
- **PostgreSQL client**: `psql` for database inspection
- **Redis CLI**: `redis-cli` for debugging

## 🚀 Quick Start

### Windows (PowerShell)

```powershell
# 1. Install Rust (if not installed)
winget install Rustlang.Rustup

# 2. Install Docker Desktop (if not installed)
winget install Docker.DockerDesktop

# 3. Clone/navigate to project directory
cd "C:\Users\yudhb\OneDrive\Desktop\agentic ai"

# 4. Install Rust dependencies and build
cargo build --release

# 5. Start all services with Docker Compose
docker-compose up --build

# 6. Access the services:
# - Orchestrator API: http://localhost:8080
# - Ingestion Service: http://localhost:8001
# - Embedding Service: http://localhost:8002
# - Vector DB Service: http://localhost:8003
# - Health checks: http://localhost:8080/health
# - Metrics: http://localhost:8080/metrics
```

### Linux/Mac (Bash)

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install Docker
# Follow official Docker installation for your OS

# 3. Build and start services
docker-compose up --build
```

## 📚 API Documentation

### Orchestrator API (`/api/v1/`)

#### 1. Ingest Document
```bash
curl -X POST http://localhost:8080/api/v1/ingest \
  -F "file=@sample.pdf" \
  -H "Content-Type: multipart/form-data"
```

**Response:**
```json
{
  "document_id": "uuid",
  "filename": "sample.pdf",
  "passages_count": 42
}
```

#### 2. Query with Streaming
```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "query": "Summarize login incidents last 30 days and propose tickets"
  }'
```

**Response:**
```json
{
  "request_id": "uuid",
  "summary": "Based on the retrieved documents, there were 15 login failures in the last 30 days [source:doc123:page2:passage5]...",
  "citations": [
    {
      "doc_id": "uuid",
      "passage_id": "uuid",
      "page": 2,
      "text": "Login failure on 2024-01-15...",
      "relevance_score": 0.92
    }
  ],
  "pending_actions": ["action_uuid"]
}
```

#### 3. Get Pending Actions
```bash
curl "http://localhost:8080/api/v1/pending?user_id=user123"
```

#### 4. Approve/Reject Action
```bash
curl -X POST http://localhost:8080/api/v1/approve \
  -H "Content-Type: application/json" \
  -d '{
    "action_id": "uuid",
    "approved": true,
    "user_signature": "John Doe - 2024-01-20T10:30:00Z"
  }'
```

#### 5. Get Source Document
```bash
curl "http://localhost:8080/api/v1/sources/{doc_id}"
```

### Microservice APIs

See individual service documentation:
- Ingestion: `http://localhost:8001/docs`
- Embedding: `http://localhost:8002/docs`
- Vector DB: `http://localhost:8003/docs`

## 🗄️ Database Schema

### Tables

- **documents**: Original file metadata
- **passages**: Chunked text with positions
- **embeddings_meta**: Vector DB references
- **requests**: User queries and status
- **tasks**: Agent execution records
- **pending_actions**: Actions awaiting approval
- **audit_logs**: Immutable provenance trail

### Migrations

Migrations run automatically on orchestrator startup via `sqlx::migrate!`.

Manual migration:
```bash
sqlx migrate run --database-url "postgres://postgres:postgres@localhost:5432/agentic_ai"
```

## 🧪 Testing

### Unit Tests (Rust)
```bash
cargo test
```

### Integration Tests
```bash
# Start services
docker-compose up -d

# Run integration tests
cargo test --test integration_tests
```

### End-to-End Demo
```bash
# Upload sample document
curl -X POST http://localhost:8080/api/v1/ingest \
  -F "file=@tests/fixtures/sample_incident_report.pdf"

# Query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"user_id": "test", "query": "Summarize incidents"}'

# Check pending actions
curl "http://localhost:8080/api/v1/pending?user_id=test"
```

## 🔧 Configuration

### Environment Variables

Create `.env` file:
```env
# Orchestrator
PORT=8080
DATABASE_URL=postgres://postgres:postgres@localhost:5432/agentic_ai
REDIS_URL=redis://localhost:6379
OPENAI_API_KEY=sk-...
JWT_SECRET=your-secret-key

# Service URLs
INGESTION_SERVICE_URL=http://localhost:8001
EMBEDDING_SERVICE_URL=http://localhost:8002
VECTOR_DB_SERVICE_URL=http://localhost:8003

# Logging
LOG_LEVEL=info
RUST_LOG=info
```

## 📊 Monitoring

### Prometheus Metrics
```bash
curl http://localhost:8080/metrics
```

**Key Metrics:**
- `http_requests_total`: Request counter
- `http_request_duration_seconds`: Latency histogram
- `agent_task_duration_seconds`: Agent execution time
- `llm_tokens_total`: Token usage
- `vector_search_duration_seconds`: Search latency

### Logs

Structured JSON logs for all services:
```bash
# View orchestrator logs
docker logs -f agentic-orchestrator

# View all logs
docker-compose logs -f
```

### Tracing (Planned)

OpenTelemetry + Jaeger for distributed tracing.

## 🔐 Security

### Current (MVP)

- API key authentication (JWT scaffolding)
- Rate limiting (basic)
- Input validation
- CORS policies
- Environment variable secrets

### Planned

- OAuth2 integration (Google, GitHub)
- HashiCorp Vault for secret management
- PII redaction pipeline
- RBAC enforcement
- Audit log immutability (implemented)

## 🚧 Roadmap

### Phase 1: MVP ✅ (Current)
- Text-only RAG
- Manual action approval
- Basic agents
- Docker Compose deployment

### Phase 2: Multimodal (In Progress)
- [ ] OCR for images (PaddleOCR/TrOCR)
- [ ] Table extraction & computation
- [ ] Audio transcription (Whisper)
- [ ] Vision-LLM integration

### Phase 3: Auto-Actions & Governance
- [ ] External connectors (JIRA, Slack, GDrive, Email)
- [ ] Sandboxed code execution
- [ ] Advanced model routing
- [ ] Multi-level approval chains

### Phase 4: Production Hardening
- [ ] Kubernetes deployment (Helm charts)
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Production vector DB (Milvus/Pinecone)
- [ ] S3 integration
- [ ] Load testing & optimization

## 📝 Development

### Project Structure

```
.
├── Cargo.toml              # Rust workspace
├── orchestrator/           # Warp API gateway
│   ├── src/
│   │   ├── main.rs
│   │   ├── api/           # REST endpoints
│   │   ├── agents/        # Agent implementations
│   │   ├── models.rs      # Data models
│   │   └── db.rs          # Database layer
│   └── migrations/        # SQL migrations
├── services/
│   ├── ingestion/         # Document extraction
│   ├── embedding/         # Vector embeddings
│   └── vector-db/         # FAISS wrapper
├── frontend/              # Next.js (planned)
├── docker-compose.yml     # Dev stack
└── README.md
```

### Adding a New Agent

1. Create module in `orchestrator/src/agents/`
2. Implement agent trait
3. Add Redis channel constant
4. Wire into orchestration flow
5. Add tests

Example:
```rust
// orchestrator/src/agents/my_agent.rs
pub struct MyAgent;

impl MyAgent {
    pub async fn execute(&self, input: &str) -> Result<String> {
        // Agent logic
        Ok("result".to_string())
    }
}
```

### Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

## 🐛 Troubleshooting

### Docker Compose Issues

```bash
# Rebuild containers
docker-compose down
docker-compose build --no-cache
docker-compose up

# Check logs
docker-compose logs <service-name>

# Reset database
docker-compose down -v  # WARNING: Deletes data
docker-compose up
```

### Rust Build Issues

```bash
# Update toolchain
rustup update

# Clean build
cargo clean
cargo build

# Check dependencies
cargo tree
```

### Port Conflicts

```bash
# Check ports in use
netstat -ano | findstr :8080  # Windows
lsof -i :8080                 # Linux/Mac

# Change ports in docker-compose.yml
```

## 📄 License

MIT License - see LICENSE file for details

## 🙏 Acknowledgments

- [Warp](https://github.com/seanmonstar/warp) - Fast async web framework
- [FastAPI](https://fastapi.tiangolo.com/) - Modern Python web framework
- [sentence-transformers](https://www.sbert.net/) - Embeddings library
- [FAISS](https://github.com/facebookresearch/faiss) - Vector similarity search

## 📧 Support

For questions or issues:
- Open a GitHub issue
- Check the [Architecture documentation](ARCHITECTURE.md)
- Review the [Deployment guide](DEPLOYMENT.md)

---

**Status**: MVP Phase 1 Complete ✅ | Phase 2 In Progress 🚧

Last Updated: 2024-12-02
