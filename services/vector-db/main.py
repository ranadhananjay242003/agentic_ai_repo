"""
Vector Database Service (FAISS wrapper with User ID Filtering)
"""
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any, Optional
import logging
import numpy as np

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='{"time": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}'
)
logger = logging.getLogger(__name__)

app = FastAPI(title="Vector Database Service", version="1.1.0")

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
            _index = faiss.IndexFlatIP(384)
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
    user_id: Optional[str] = None  # <--- NEW FIELD FOR FILTERING

class SearchResult(BaseModel):
    vector_db_id: str
    score: float
    metadata: Dict[str, Any]

class SearchResponse(BaseModel):
    results: List[SearchResult]
    search_type: str

@app.get("/health")
async def health_check():
    return {"status": "healthy", "service": "vector-db"}

@app.post("/index/add")
async def add_vectors(request: AddVectorsRequest):
    """Add vectors to the index"""
    if len(request.vectors) != len(request.metadata):
        raise HTTPException(status_code=400, detail="Vectors and metadata length mismatch")
    
    if not request.vectors:
        raise HTTPException(status_code=400, detail="No vectors provided")
    
    try:
        index = get_faiss_index()
        vectors_np = np.array(request.vectors, dtype=np.float32)
        
        start_id = index.ntotal
        index.add(vectors_np)
        
        for i, metadata in enumerate(request.metadata):
            vector_db_id = f"vec_{start_id + i}"
            _documents.append({
                "vector_db_id": vector_db_id,
                "metadata": metadata
            })
            _id_to_idx[vector_db_id] = start_id + i
        
        logger.info(f"Added {len(request.vectors)} vectors to index")
        return {"added": len(request.vectors), "total_vectors": index.ntotal}
    
    except Exception as e:
        logger.error(f"Error adding vectors: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Failed to add vectors: {str(e)}")

@app.post("/search/hybrid", response_model=SearchResponse)
async def hybrid_search(request: SearchRequest):
    """
    Hybrid search with User ID Filtering
    """
    try:
        index = get_faiss_index()
        
        if index.ntotal == 0:
            return SearchResponse(results=[], search_type="hybrid")
        
        query_vector_np = np.array([request.query_vector], dtype=np.float32)
        
        # 1. Fetch MORE candidates than needed (e.g. 5x) to allow for filtering
        # If user wants top 3, we fetch top 15, then filter out other users' docs
        fetch_k = min(request.top_k * 10, index.ntotal)
        distances, indices = index.search(query_vector_np, fetch_k)
        
        results = []
        for i, (dist, idx) in enumerate(zip(distances[0], indices[0])):
            if idx < len(_documents):
                doc = _documents[idx]
                
                # --- FILTERING LOGIC ---
                # If the search request has a user_id, ensure the doc matches it
                doc_user_id = doc["metadata"].get("user_id")
                if request.user_id and doc_user_id:
                    if doc_user_id != request.user_id:
                        continue # Skip this document, it belongs to someone else
                # -----------------------

                # Calculate Scores (Hybrid)
                lexical_score = 0.0
                if request.hybrid and request.query_text:
                    text = doc["metadata"].get("text", "").lower()
                    query_words = set(request.query_text.lower().split())
                    text_words = set(text.split())
                    if text_words and query_words:
                        lexical_score = len(query_words & text_words) / len(query_words)
                
                vector_score = float(dist)
                final_score = vector_score
                if request.hybrid and request.query_text:
                    final_score = 0.7 * vector_score + 0.3 * lexical_score
                
                results.append({
                    "vector_db_id": doc["vector_db_id"],
                    "score": final_score,
                    "metadata": doc["metadata"]
                })
        
        # Sort and limit
        results.sort(key=lambda x: x["score"], reverse=True)
        results = results[:request.top_k]
        
        logger.info(f"Search returned {len(results)} valid results for user {request.user_id}")
        
        return SearchResponse(results=[SearchResult(**r) for r in results], search_type="hybrid")
    
    except Exception as e:
        logger.error(f"Search error: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Search failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8003)