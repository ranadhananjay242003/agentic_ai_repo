"""
Embedding Service
Generates vector embeddings for text passages
"""
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List
import logging
import numpy as np

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='{"time": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}'
)
logger = logging.getLogger(__name__)

app = FastAPI(title="Embedding Service", version="1.0.0")

# Load model (lazy loading to speed up startup)
_model = None

def get_model():
    global _model
    if _model is None:
        try:
            from sentence_transformers import SentenceTransformer
            logger.info("Loading embedding model: all-MiniLM-L6-v2")
            _model = SentenceTransformer('all-MiniLM-L6-v2')
            logger.info("Model loaded successfully")
        except ImportError:
            logger.error("sentence-transformers not installed")
            raise HTTPException(status_code=500, detail="sentence-transformers not available")
    return _model

class EmbedRequest(BaseModel):
    texts: List[str]
    normalize: bool = True

class EmbedResponse(BaseModel):
    embeddings: List[List[float]]
    model: str
    dimensions: int

class ModelInfo(BaseModel):
    model: str
    dimensions: int
    max_seq_length: int

@app.get("/health")
async def health_check():
    return {"status": "healthy", "service": "embedding"}

@app.get("/model-info", response_model=ModelInfo)
async def model_info():
    """Get embedding model information"""
    model = get_model()
    return ModelInfo(
        model="all-MiniLM-L6-v2",
        dimensions=384,
        max_seq_length=model.max_seq_length
    )

@app.post("/embed", response_model=EmbedResponse)
async def embed_texts(request: EmbedRequest):
    """
    Generate embeddings for a batch of texts
    """
    if not request.texts:
        raise HTTPException(status_code=400, detail="No texts provided")
    
    if len(request.texts) > 100:
        raise HTTPException(status_code=400, detail="Maximum 100 texts per request")
    
    logger.info(f"Embedding {len(request.texts)} texts")
    
    try:
        model = get_model()
        embeddings = model.encode(
            request.texts,
            normalize_embeddings=request.normalize,
            show_progress_bar=False
        )
        
        # Convert to list for JSON serialization
        embeddings_list = embeddings.tolist()
        
        logger.info(f"Generated {len(embeddings_list)} embeddings")
        
        return EmbedResponse(
            embeddings=embeddings_list,
            model="all-MiniLM-L6-v2",
            dimensions=len(embeddings_list[0]) if embeddings_list else 0
        )
    
    except Exception as e:
        logger.error(f"Embedding error: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Embedding failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8002)
