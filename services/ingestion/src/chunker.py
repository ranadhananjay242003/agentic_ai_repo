"""
Passage chunking with overlap
"""
from typing import List, Dict, Any
import logging

logger = logging.getLogger(__name__)

class PassageChunker:
    def __init__(self, chunk_size: int = 512, overlap: int = 50):
        """
        chunk_size: target number of characters per chunk
        overlap: number of characters to overlap between chunks
        """
        self.chunk_size = chunk_size
        self.overlap = overlap
    
    def chunk(self, text: str, metadata: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Chunk text into overlapping passages
        """
        passages = []
        text_length = len(text)
        
        if text_length == 0:
            return passages
        
        passage_id = 0
        char_start = 0
        
        while char_start < text_length:
            char_end = min(char_start + self.chunk_size, text_length)
            
            # Extract passage text
            passage_text = text[char_start:char_end].strip()
            
            # Skip empty passages
            if not passage_text:
                char_start = char_end
                continue
            
            # Estimate page number based on char position (rough approximation)
            # Assuming ~3000 chars per page
            estimated_page = None
            if "total_pages" in metadata and metadata["total_pages"] > 0:
                estimated_page = int((char_start / text_length) * metadata["total_pages"]) + 1
            
            passages.append({
                "passage_id": passage_id,
                "text": passage_text,
                "char_start": char_start,
                "char_end": char_end,
                "page": estimated_page,
                "metadata": {
                    "length": len(passage_text),
                    "format": metadata.get("format", "unknown")
                }
            })
            
            passage_id += 1
            
            # Move start position forward by (chunk_size - overlap)
            char_start += self.chunk_size - self.overlap
        
        logger.info(f"Chunked {text_length} chars into {len(passages)} passages")
        return passages
