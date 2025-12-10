"""
Document Ingestion Service
Extracts text and metadata from various document formats (with Advanced Table Extraction)
"""
from fastapi import FastAPI, File, UploadFile, HTTPException
from pydantic import BaseModel
from typing import List, Optional, Dict, Any
import logging
import sys
import io
import os
import requests
import base64
import json
from pathlib import Path

# --- NEW TABLE IMPORTS ---
import camelot
import pandas as pd

# --- EXISTING IMPORTS ---
import pytesseract
from PIL import Image

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

from extractors import PDFExtractor, DOCXExtractor, PPTXExtractor, CSVExtractor
from chunker import PassageChunker

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(title="Document Ingestion Service", version="1.6.0")

# --- NEW TABLE EXTRACTOR CLASS ---
class PDFTableExtractor:
    def extract_tables_to_text(self, pdf_bytes: bytes) -> str:
        """Extracts tables from PDF bytes and returns them as a structured string."""
        try:
            # Save bytes to a temporary file for camelot
            temp_path = Path("/tmp") / f"temp_{os.getpid()}.pdf"
            with open(temp_path, "wb") as f:
                f.write(pdf_bytes)
            
            logger.info(f"Extracting tables from {temp_path}...")
            # Use 'lattice' method for structured tables
            tables = camelot.read_pdf(str(temp_path), flavor='lattice', pages='all')
            
            structured_text = []
            if tables.n > 0:
                for i, table in enumerate(tables):
                    df: pd.DataFrame = table.df
                    
                    # Convert DataFrame to a clean, searchable markdown-like text
                    table_markdown = df.to_markdown(index=False)
                    
                    structured_text.append(f"\n--- TABLE {i+1} START ---\n{table_markdown}\n--- TABLE {i+1} END ---\n")
                
                logger.info(f"Successfully extracted {tables.n} tables.")
                return "\n".join(structured_text)
            
            return ""

        except Exception as e:
            logger.error(f"Camelot Table Extraction Failed: {e}")
            return f"\n--- TABLE EXTRACTION FAILED: {str(e)} ---\n"
        finally:
            if temp_path.exists():
                os.remove(temp_path)

# --- EXISTING EXTRACTORS (Truncated for brevity, assuming you merge this in) ---
# ... (ImageExtractor, AudioExtractor classes remain the same) ...

class ImageExtractor:
    # (Content remains the same as previous step's ImageExtractor)
    def extract(self, content: bytes, filename: str) -> tuple[str, dict]:
        # ... (Same logic as before, but ensure you include ALL dependencies) ...
        # Simplified for demonstration:
        try:
            image = Image.open(io.BytesIO(content))
            ocr_text = pytesseract.image_to_string(image)
            return ocr_text, {"format": "image"}
        except:
            return "[Image extraction failed]", {"format": "image"}

class AudioExtractor:
    # (Content remains the same as previous step's AudioExtractor)
    def extract(self, content: bytes, filename: str) -> tuple[str, dict]:
        return "Audio transcription placeholder", {"format": "audio"}

# Initialize extractors
pdf_extractor = PDFExtractor()
docx_extractor = DOCXExtractor()
pptx_extractor = PPTXExtractor()
csv_extractor = CSVExtractor()
image_extractor = ImageExtractor()
audio_extractor = AudioExtractor() 
table_extractor = PDFTableExtractor() # NEW INITIALIZATION

chunker = PassageChunker(chunk_size=1024, overlap=50)

class Passage(BaseModel):
    passage_id: int
    text: str
    page: Optional[int]
    char_start: int
    char_end: int
    metadata: Dict[str, Any]

class ExtractionResponse(BaseModel):
    filename: str
    content_type: str
    total_chars: int
    passages: List[Passage]

@app.post("/extract", response_model=ExtractionResponse)
async def extract_document(file: UploadFile = File(...)):
    try:
        content = await file.read()
        text = ""
        metadata = {}

        if file.content_type == "application/pdf" or file.filename.endswith(".pdf"):
            # --- MODIFIED: Extract text AND tables ---
            base_text, metadata = pdf_extractor.extract(content)
            table_text = table_extractor.extract_tables_to_text(content)
            
            # Combine text and tables
            text = f"{base_text}\n\n{table_text}"
            metadata["has_tables"] = bool(table_text)
        
        elif file.filename.endswith((".docx", ".doc")):
            text, metadata = docx_extractor.extract(content)
        elif file.filename.endswith(".csv"):
            text, metadata = csv_extractor.extract(content)
        elif file.content_type.startswith("image/") or file.filename.lower().endswith((".png", ".jpg", ".jpeg", ".webp")):
            text, metadata = image_extractor.extract(content, file.filename)
        elif file.content_type.startswith("audio/") or file.filename.lower().endswith((".mp3", ".wav", ".m4a", ".ogg")):
            text, metadata = audio_extractor.extract(content, file.filename)
        elif file.content_type == "text/plain" or file.filename.endswith(".txt"):
            text = content.decode("utf-8")
            metadata = {"format": "txt"}
        else:
            raise HTTPException(status_code=400, detail=f"Unsupported file type: {file.content_type}")
        
        # ... (rest of the code for chunking and returning the response) ...
        if not text or not text.strip(): text = "[No text found]"
        passages_data = chunker.chunk(text, metadata)
        passages = [Passage(passage_id=p["passage_id"], text=p["text"], page=p.get("page"), char_start=p["char_start"], char_end=p["char_end"], metadata=p.get("metadata", {})) for p in passages_data]
        
        return ExtractionResponse(filename=file.filename, content_type=file.content_type or "unknown", total_chars=len(text), passages=passages)
        
    except Exception as e:
        logger.error(f"Error: {str(e)}")
        raise HTTPException(status_code=500, detail=f"Extraction failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)