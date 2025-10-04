# RAG-Redis Integration Design for mistral.rs Context Management

## Executive Summary

This document outlines a comprehensive design for integrating RAG-Redis as the primary context management system for the mistral.rs project. The system will ingest project documentation, provide intelligent context retrieval for agents, and maintain strict performance constraints to prevent system overload.

## 1. Document Ingestion Architecture

### 1.1 Document Selection Strategy

**Primary Documents** (High Priority):

- `.claude/CLAUDE.md` - Critical build instructions
- `CLAUDE.md` - Project-specific guidance
- `docs/*.md` - API documentation (70+ files)
- `README.md` - Project overview
- `AGENT_*.md` - Agent implementation docs
- `mistralrs-pyo3/API.md` - Python API reference

**Secondary Documents** (Medium Priority):

- `examples/*/README.md` - Usage examples
- `.github/*.md` - CI/CD and contribution guides
- `tests/*/README.md` - Testing documentation

**Excluded Documents**:

- Issue templates
- Temporary context files
- Generated documentation

### 1.2 Document Chunking Strategy

```rust
// Document chunking configuration
pub struct ChunkingConfig {
    // Semantic chunking with overlap
    pub max_chunk_size: usize,      // 1500 tokens (~6000 chars)
    pub chunk_overlap: usize,        // 200 tokens overlap
    pub min_chunk_size: usize,       // 100 tokens minimum

    // Smart splitting at:
    // - Heading boundaries (# ## ###)
    // - Code block boundaries
    // - Paragraph boundaries
    pub preserve_code_blocks: bool,  // true
    pub preserve_tables: bool,       // true

    // Metadata extraction
    pub extract_headings: bool,      // true - for hierarchical context
    pub extract_code_lang: bool,     // true - for language-specific queries
}
```

### 1.3 Embedding Strategy

**Hybrid Embedding Approach**:

1. **Dense Embeddings**: Using BGE-small-en-v1.5 (384 dimensions)
1. **Sparse Embeddings**: BM25 for keyword matching
1. **Code Embeddings**: CodeBERT for code-specific chunks

```rust
pub enum EmbeddingType {
    TextContent {
        model: String,  // "bge-small-en-v1.5"
        dims: usize,    // 384
    },
    CodeContent {
        model: String,  // "codebert-base"
        dims: usize,    // 768
        language: String,
    },
    Hybrid {
        dense: Box<EmbeddingType>,
        sparse: BM25Config,
    }
}
```

## 2. Query API Design

### 2.1 Core Query Interface

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ContextQuery {
    /// The query text
    pub query: String,

    /// Filter by document type
    pub doc_type: Option<DocType>,

    /// Maximum number of results
    pub limit: usize,  // Default: 5, Max: 10

    /// Minimum similarity threshold
    pub threshold: f32,  // Default: 0.7

    /// Include code examples
    pub include_code: bool,

    /// Specific to a crate/module
    pub scope: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DocType {
    BuildInstructions,
    ApiDocumentation,
    Examples,
    Architecture,
    Testing,
    Configuration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContextResult {
    pub chunks: Vec<DocumentChunk>,
    pub total_tokens: usize,
    pub query_time_ms: u64,
    pub cache_hit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentChunk {
    pub content: String,
    pub source: String,
    pub section: Option<String>,
    pub relevance_score: f32,
    pub metadata: ChunkMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChunkMetadata {
    pub doc_type: DocType,
    pub language: Option<String>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub token_count: usize,
}

#[async_trait]
pub trait RagContextProvider: Send + Sync {
    /// Query for relevant context
    async fn query_context(&self, query: ContextQuery) -> Result<ContextResult>;

    /// Ingest a new document
    async fn ingest_document(&self, path: &str, doc_type: DocType) -> Result<()>;

    /// Update existing document
    async fn update_document(&self, path: &str) -> Result<()>;

    /// Get statistics
    async fn get_stats(&self) -> Result<RagStats>;
}
```

### 2.2 Agent Integration Interface

```rust
/// Simplified interface for agent usage
pub struct AgentContextManager {
    provider: Arc<dyn RagContextProvider>,
    cache: Arc<RwLock<LruCache<String, ContextResult>>>,
    rate_limiter: Arc<RateLimiter>,
}

impl AgentContextManager {
    /// Get build instructions context
    pub async fn get_build_context(&self,
        target: &str  // e.g., "cuda", "metal", "cpu"
    ) -> Result<String> {
        let query = ContextQuery {
            query: format!("build instructions for {}", target),
            doc_type: Some(DocType::BuildInstructions),
            limit: 3,
            threshold: 0.8,
            include_code: true,
            scope: None,
        };

        self.query_with_cache(query).await
    }

    /// Get API documentation
    pub async fn get_api_docs(&self,
        module: &str,
        function: Option<&str>
    ) -> Result<String> {
        let query_text = match function {
            Some(f) => format!("{} {} function", module, f),
            None => format!("{} module documentation", module),
        };

        let query = ContextQuery {
            query: query_text,
            doc_type: Some(DocType::ApiDocumentation),
            limit: 5,
            threshold: 0.75,
            include_code: true,
            scope: Some(module.to_string()),
        };

        self.query_with_cache(query).await
    }

    /// Get relevant examples
    pub async fn get_examples(&self,
        topic: &str
    ) -> Result<Vec<String>> {
        let query = ContextQuery {
            query: format!("example code for {}", topic),
            doc_type: Some(DocType::Examples),
            limit: 3,
            threshold: 0.7,
            include_code: true,
            scope: None,
        };

        let result = self.query_with_cache(query).await?;
        Ok(result.chunks.iter()
            .filter(|c| c.metadata.language.is_some())
            .map(|c| c.content.clone())
            .collect())
    }
}
```

## 3. Performance Optimization Strategy

### 3.1 Rate Limiting

```rust
pub struct RateLimiter {
    /// Max queries per minute
    max_qpm: u32,  // 60

    /// Max concurrent queries
    max_concurrent: u32,  // 3

    /// Sliding window for rate tracking
    window: Arc<RwLock<VecDeque<Instant>>>,

    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub async fn acquire(&self) -> Result<RateLimitGuard> {
        // Wait for semaphore permit (max 3 concurrent)
        let permit = self.semaphore.acquire().await?;

        // Check rate limit (60/minute)
        let mut window = self.window.write().await;
        let now = Instant::now();

        // Remove entries older than 1 minute
        while let Some(front) = window.front() {
            if now.duration_since(*front) > Duration::from_secs(60) {
                window.pop_front();
            } else {
                break;
            }
        }

        // Check if under limit
        if window.len() >= self.max_qpm as usize {
            let wait_time = Duration::from_secs(60) - now.duration_since(window[0]);
            tokio::time::sleep(wait_time).await;
        }

        window.push_back(now);

        Ok(RateLimitGuard { _permit: permit })
    }
}
```

### 3.2 Multi-Tier Caching

```rust
pub struct CacheStrategy {
    /// L1: In-memory LRU cache (hot queries)
    l1_cache: Arc<RwLock<LruCache<String, ContextResult>>>,
    l1_size: usize,  // 100 entries
    l1_ttl: Duration,  // 5 minutes

    /// L2: Redis cache (warm queries)
    l2_cache: Arc<RedisClient>,
    l2_ttl: Duration,  // 1 hour

    /// L3: Disk cache (cold queries)
    l3_cache: Option<DiskCache>,
    l3_ttl: Duration,  // 24 hours
}

impl CacheStrategy {
    pub async fn get(&self, key: &str) -> Option<ContextResult> {
        // Check L1 (fastest)
        if let Some(result) = self.l1_cache.read().await.get(key) {
            return Some(result.clone());
        }

        // Check L2 (Redis)
        if let Ok(Some(data)) = self.l2_cache.get(key).await {
            if let Ok(result) = serde_json::from_str::<ContextResult>(&data) {
                // Promote to L1
                self.l1_cache.write().await.put(key.to_string(), result.clone());
                return Some(result);
            }
        }

        // Check L3 (disk)
        if let Some(ref disk_cache) = self.l3_cache {
            if let Ok(result) = disk_cache.get(key).await {
                // Promote to L1 and L2
                self.promote_to_cache(&result, key).await;
                return Some(result);
            }
        }

        None
    }

    pub async fn put(&self, key: String, value: ContextResult) {
        // Write to all cache levels
        self.l1_cache.write().await.put(key.clone(), value.clone());

        if let Ok(json) = serde_json::to_string(&value) {
            let _ = self.l2_cache.set_ex(&key, &json, self.l2_ttl.as_secs()).await;
        }

        if let Some(ref disk_cache) = self.l3_cache {
            let _ = disk_cache.put(&key, &value).await;
        }
    }
}
```

### 3.3 Connection Pooling

```rust
pub struct ConnectionPool {
    /// Redis connection pool
    redis_pool: bb8::Pool<RedisConnectionManager>,

    /// Maximum pool size
    max_size: u32,  // 10

    /// Connection timeout
    connection_timeout: Duration,  // 5 seconds

    /// Idle connection timeout
    idle_timeout: Duration,  // 300 seconds
}

impl ConnectionPool {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let manager = RedisConnectionManager::new(redis_url)?;

        let pool = bb8::Pool::builder()
            .max_size(10)
            .min_idle(Some(2))
            .connection_timeout(Duration::from_secs(5))
            .idle_timeout(Some(Duration::from_secs(300)))
            .build(manager)
            .await?;

        Ok(Self {
            redis_pool: pool,
            max_size: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
        })
    }

    pub async fn get_connection(&self) -> Result<PooledConnection> {
        self.redis_pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get connection: {}", e))
    }
}
```

## 4. Rust Implementation Examples

### 4.1 MCP Server Integration

```rust
use mistralrs_mcp::client::{McpClient, McpServerConnection};
use serde_json::json;

pub struct RagMcpClient {
    mcp_client: Arc<McpClient>,
    server_name: String,
}

impl RagMcpClient {
    pub async fn new(config: McpClientConfig) -> Result<Self> {
        let mut client = McpClient::new(config);
        client.initialize().await?;

        Ok(Self {
            mcp_client: Arc::new(client),
            server_name: "RAG Redis".to_string(),
        })
    }

    /// Ingest documents through MCP
    pub async fn ingest_documents(&self, paths: Vec<String>) -> Result<()> {
        let args = json!({
            "documents": paths,
            "chunking": {
                "max_size": 1500,
                "overlap": 200,
                "preserve_code": true
            },
            "embedding": {
                "model": "bge-small-en-v1.5",
                "batch_size": 32
            }
        });

        self.mcp_client
            .call_tool("ingest_documents", args)
            .await?;

        Ok(())
    }

    /// Query for context
    pub async fn query(&self, query: ContextQuery) -> Result<ContextResult> {
        let args = serde_json::to_value(query)?;

        let response = self.mcp_client
            .call_tool("query_context", args)
            .await?;

        serde_json::from_str(&response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Update embeddings for changed documents
    pub async fn update_embeddings(&self, paths: Vec<String>) -> Result<()> {
        let args = json!({
            "documents": paths,
            "force": false  // Only update if changed
        });

        self.mcp_client
            .call_tool("update_embeddings", args)
            .await?;

        Ok(())
    }
}
```

### 4.2 Integration with mistral.rs Server

```rust
// In mistralrs-server/src/main.rs or similar
use crate::rag::{AgentContextManager, RagMcpClient};

pub struct EnhancedMistralServer {
    // Existing fields...

    /// RAG context manager
    context_manager: Arc<AgentContextManager>,
}

impl EnhancedMistralServer {
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // Initialize RAG client
        let rag_client = RagMcpClient::new(config.mcp_config).await?;

        // Initialize context manager
        let context_manager = AgentContextManager::new(
            Arc::new(rag_client),
            CacheConfig {
                l1_size: 100,
                l1_ttl: Duration::from_secs(300),
                l2_ttl: Duration::from_secs(3600),
                enable_disk_cache: true,
            },
            RateLimiterConfig {
                max_qpm: 60,
                max_concurrent: 3,
            },
        ).await?;

        // Ingest initial documents
        context_manager.ingest_project_docs().await?;

        Ok(Self {
            // ... existing initialization
            context_manager: Arc::new(context_manager),
        })
    }

    /// Enhanced completion with context
    pub async fn complete_with_context(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        // Extract context needs from request
        let context_query = self.extract_context_needs(&request)?;

        // Get relevant context
        let context = self.context_manager
            .query_context(context_query)
            .await?;

        // Inject context into prompt
        let enhanced_request = self.inject_context(request, context)?;

        // Process with existing pipeline
        self.process_completion(enhanced_request).await
    }
}
```

### 4.3 Batch Ingestion Script

```rust
use tokio::fs;
use glob::glob;

pub async fn ingest_project_documentation(
    rag_client: &RagMcpClient,
) -> Result<()> {
    // Collect all documentation files
    let mut documents = Vec::new();

    // Primary documentation
    for pattern in &[
        "*.md",
        ".claude/*.md",
        "docs/*.md",
        "examples/*/README.md",
        "mistralrs-*/README.md",
    ] {
        for entry in glob(pattern)? {
            if let Ok(path) = entry {
                documents.push(path.to_string_lossy().to_string());
            }
        }
    }

    // Batch ingest with progress tracking
    const BATCH_SIZE: usize = 10;
    let total = documents.len();

    for (i, chunk) in documents.chunks(BATCH_SIZE).enumerate() {
        println!("Ingesting batch {}/{}", i + 1, (total + BATCH_SIZE - 1) / BATCH_SIZE);

        rag_client.ingest_documents(chunk.to_vec()).await?;

        // Rate limit between batches
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("Successfully ingested {} documents", total);

    Ok(())
}
```

## 5. Monitoring and Metrics

### 5.1 Performance Metrics

```rust
#[derive(Debug, Serialize)]
pub struct RagMetrics {
    /// Query performance
    pub query_metrics: QueryMetrics,

    /// Cache performance
    pub cache_metrics: CacheMetrics,

    /// System resources
    pub resource_metrics: ResourceMetrics,
}

#[derive(Debug, Serialize)]
pub struct QueryMetrics {
    pub total_queries: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub queries_per_minute: f64,
    pub error_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct CacheMetrics {
    pub l1_hit_rate: f64,  // Target: >70%
    pub l2_hit_rate: f64,  // Target: >50%
    pub l3_hit_rate: f64,  // Target: >30%
    pub total_hit_rate: f64,  // Target: >70%
    pub cache_size_mb: f64,
    pub eviction_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct ResourceMetrics {
    pub memory_usage_mb: f64,
    pub redis_connections: u32,
    pub pending_queries: u32,
    pub embedding_queue_size: u32,
}
```

### 5.2 Monitoring Implementation

```rust
use prometheus::{Counter, Histogram, Gauge, register_counter, register_histogram};

pub struct MetricsCollector {
    query_counter: Counter,
    query_latency: Histogram,
    cache_hits: Counter,
    cache_misses: Counter,
    memory_gauge: Gauge,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            query_counter: register_counter!("rag_queries_total", "Total RAG queries")?,
            query_latency: register_histogram!(
                "rag_query_latency_seconds",
                "RAG query latency in seconds"
            )?,
            cache_hits: register_counter!("rag_cache_hits_total", "Cache hits")?,
            cache_misses: register_counter!("rag_cache_misses_total", "Cache misses")?,
            memory_gauge: register_gauge!("rag_memory_usage_bytes", "Memory usage")?,
        })
    }

    pub fn record_query(&self, latency: Duration, cache_hit: bool) {
        self.query_counter.inc();
        self.query_latency.observe(latency.as_secs_f64());

        if cache_hit {
            self.cache_hits.inc();
        } else {
            self.cache_misses.inc();
        }
    }
}
```

## 6. Implementation Plan

### Phase 1: Infrastructure Setup (Week 1)

1. Configure RAG-Redis server with proper environment variables
1. Set up Redis with persistence and appropriate memory limits
1. Create connection pool and rate limiter implementations
1. Implement basic MCP client integration

### Phase 2: Document Ingestion (Week 2)

1. Implement document discovery and classification
1. Create chunking pipeline with metadata extraction
1. Set up embedding generation (BGE-small for text, CodeBERT for code)
1. Batch ingest all project documentation

### Phase 3: Query API Implementation (Week 3)

1. Implement core query interface
1. Create agent-specific helper methods
1. Set up multi-tier caching system
1. Add query optimization and ranking

### Phase 4: Integration & Testing (Week 4)

1. Integrate with mistral.rs server
1. Add monitoring and metrics collection
1. Performance testing and optimization
1. Documentation and deployment

## 7. Configuration

### 7.1 Environment Variables

```bash
# RAG-Redis Configuration
REDIS_URL=redis://127.0.0.1:6379
RAG_DATA_DIR=T:/projects/rust-mistral/mistral.rs/rag-data
EMBEDDING_CACHE_DIR=T:/projects/rust-mistral/mistral.rs/rag-cache
LOG_DIR=T:/projects/rust-mistral/mistral.rs/logs/rag

# Performance Limits
RAG_MAX_QPM=60
RAG_MAX_CONCURRENT=3
RAG_CACHE_SIZE_MB=500
RAG_QUERY_TIMEOUT_MS=500

# Model Configuration
EMBEDDING_MODEL=bge-small-en-v1.5
CODE_EMBEDDING_MODEL=codebert-base
EMBEDDING_BATCH_SIZE=32
```

### 7.2 Configuration File (rag-config.toml)

```toml
[ingestion]
chunk_size = 1500
chunk_overlap = 200
min_chunk_size = 100
preserve_code_blocks = true
preserve_tables = true

[embedding]
text_model = "bge-small-en-v1.5"
code_model = "codebert-base"
batch_size = 32
cache_embeddings = true

[query]
default_limit = 5
max_limit = 10
default_threshold = 0.7
include_metadata = true

[cache]
l1_size = 100
l1_ttl_seconds = 300
l2_ttl_seconds = 3600
l3_ttl_seconds = 86400
enable_disk_cache = true

[rate_limit]
max_queries_per_minute = 60
max_concurrent = 3
burst_size = 10

[monitoring]
enable_metrics = true
metrics_port = 9090
log_level = "info"
```

## 8. Success Criteria

### Performance Targets

- **Query Latency**: P95 < 500ms
- **Cache Hit Rate**: > 70%
- **Memory Usage**: < 1GB for cache
- **Concurrent Queries**: Support 3 simultaneous
- **Query Rate**: Support 60 QPM sustained

### Quality Metrics

- **Relevance Score**: Average > 0.75
- **Context Coverage**: > 90% of queries return useful context
- **Update Latency**: < 5 minutes for document changes
- **System Uptime**: > 99.9%

## 9. Security Considerations

1. **Input Sanitization**: Validate all queries to prevent injection
1. **Rate Limiting**: Per-client rate limits to prevent abuse
1. **Access Control**: Scope queries to appropriate documents
1. **Encryption**: TLS for Redis connections, encrypted cache on disk
1. **Audit Logging**: Log all queries with timestamps and client IDs

## 10. Future Enhancements

1. **Semantic Caching**: Cache similar queries together
1. **Query Expansion**: Automatically expand queries with synonyms
1. **Feedback Loop**: Learn from user interactions to improve ranking
1. **Multi-Modal**: Support for diagram and image context
1. **Incremental Updates**: Real-time document change detection
1. **Distributed Cache**: Redis Cluster for horizontal scaling
