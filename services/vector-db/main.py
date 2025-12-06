"""
Vector Database Service (FAISS wrapper with hybrid search)
"""
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any, Optional
import logging
import numpy as np
import json
from pathlib import Path

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='{"time": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}'
)
logger = logging.getLogger(__name__)

app = FastAPI(title="Vector Database Service", version="1.0.0")

# Global index storage
_index = None
_documents = []  # Store document metadata
_id_to_idx = {}  # Map vector_db_id to index position

def get_faiss_index():
    global _index
    if _index is None:
        try:
            import faiss
            # Initialize with 384 dimensions (all-MiniLM-L6-v2)
            _index = faiss.IndexFlatIP(384)  # Inner product for cosine similarity (if normalized)
            logger.info("FAISS index initialized")
        except ImportError:
            logger.error("faiss-cpu not installed")
            raise HTTPException(status_code=500, detail="FAISS not available")
    return _index

class AddVectorsRequest(BaseModel):
    vectors: List[List[float]]
    metadata: List[Dict[str, Any]]

class SearchRequest(BaseModel):
    query_vector: List[float]
    query_text: Optional[str] = None
    top_k: int = 10
    hybrid: bool = True

class SearchResult(BaseModel):
    vector_db_id: str
    score: float
    metadata: Dict[str, Any]

class SearchResponse(BaseModel):
    results: List[SearchResult]
    search_type: str

class IndexStats(BaseModel):
    total_vectors: int
    dimensions: int
    index_type: str

@app.get("/health")
async def health_check():
    return {"status": "healthy", "service": "vector-db"}

@app.get("/index/stats", response_model=IndexStats)
async def get_stats():
    """Get index statistics"""
    index = get_faiss_index()
    return IndexStats(
        total_vectors=index.ntotal,
        dimensions=384,
        index_type="IndexFlatIP"
    )

@app.post("/index/add")
async def add_vectors(request: AddVectorsRequest):
    """Add vectors to the index"""
    if len(request.vectors) != len(request.metadata):
        raise HTTPException(status_code=400, detail="Vectors and metadata length mismatch")
    
    if not request.vectors:
        raise HTTPException(status_code=400, detail="No vectors provided")
    
    try:
        index = get_faiss_index()
        
        # Convert to numpy array
        vectors_np = np.array(request.vectors, dtype=np.float32)
        
        # Validate dimensions
        if vectors_np.shape[1] != 384:
            raise HTTPException(status_code=400, detail=f"Expected 384 dimensions, got {vectors_np.shape[1]}")
        
        # Add to index
        start_id = index.ntotal
        index.add(vectors_np)
        
        # Store metadata
        for i, metadata in enumerate(request.metadata):
            vector_db_id = f"vec_{start_id + i}"
            _documents.append({
                "vector_db_id": vector_db_id,
                "metadata": metadata
            })
            _id_to_idx[vector_db_id] = start_id + i
        
        logger.info(f"Added {len(request.vectors)} vectors to index")
        
        return {
            "added": len(request.vectors),
            "total_vectors": index.ntotal,
            "ids": [f"vec_{start_id + i}" for i in range(len(request.vectors))]
        }
    
    except Exception as e:
        logger.error(f"Error adding vectors: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Failed to add vectors: {str(e)}")

@app.post("/search/hybrid", response_model=SearchResponse)
async def hybrid_search(request: SearchRequest):
    """
    Hybrid search combining vector similarity and lexical matching
    """
    try:
        index = get_faiss_index()
        
        if index.ntotal == 0:
            logger.warning("Index is empty")
            return SearchResponse(results=[], search_type="hybrid")
        
        # Vector search
        query_vector_np = np.array([request.query_vector], dtype=np.float32)
        
        # Validate dimensions
        if query_vector_np.shape[1] != 384:
            raise HTTPException(status_code=400, detail=f"Expected 384 dimensions, got {query_vector_np.shape[1]}")
        
        k = min(request.top_k * 2, index.ntotal)  # Get more for reranking
        distances, indices = index.search(query_vector_np, k)
        
        # Collect results
        results = []
        for i, (dist, idx) in enumerate(zip(distances[0], indices[0])):
            if idx < len(_documents):
                doc = _documents[idx]
                
                # Compute lexical score if query_text provided (simple BM25 approximation)
                lexical_score = 0.0
                if request.hybrid and request.query_text:
                    text = doc["metadata"].get("text", "").lower()
                    query_words = set(request.query_text.lower().split())
                    text_words = set(text.split())
                    if text_words:
                        lexical_score = len(query_words & text_words) / len(query_words)
                
                # Hybrid score (weighted combination)
                vector_score = float(dist)
                if request.hybrid and request.query_text:
                    final_score = 0.7 * vector_score + 0.3 * lexical_score
                else:
                    final_score = vector_score
                
                results.append({
                    "vector_db_id": doc["vector_db_id"],
                    "score": final_score,
                    "metadata": doc["metadata"]
                })
        
        # Sort by final score
        results.sort(key=lambda x: x["score"], reverse=True)
        results = results[:request.top_k]
        
        logger.info(f"Hybrid search returned {len(results)} results")
        
        return SearchResponse(
            results=[SearchResult(**r) for r in results],
            search_type="hybrid" if request.hybrid else "vector"
        )
    
    except Exception as e:
        logger.error(f"Search error: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Search failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8003)
