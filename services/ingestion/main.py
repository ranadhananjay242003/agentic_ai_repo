"""
Document Ingestion Service
Extracts text and metadata from various document formats (Text + OCR)
"""
from fastapi import FastAPI, File, UploadFile, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from typing import List, Optional, Dict, Any
import logging
import sys
import io
from pathlib import Path

# --- OCR IMPORTS ---
import pytesseract
from PIL import Image

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

from extractors import PDFExtractor, DOCXExtractor, PPTXExtractor, CSVExtractor
from chunker import PassageChunker

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='{"time": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}'
)
logger = logging.getLogger(__name__)

app = FastAPI(title="Document Ingestion Service", version="1.1.0")

# --- CUSTOM IMAGE EXTRACTOR ---
class ImageExtractor:
    """Extracts text from images using Tesseract OCR"""
    def extract(self, content: bytes) -> tuple[str, dict]:
        try:
            image = Image.open(io.BytesIO(content))
            # Extract text
            text = pytesseract.image_to_string(image)
            # Metadata
            metadata = {
                "format": image.format,
                "width": image.width,
                "height": image.height,
                "mode": image.mode,
                "ocr_engine": "tesseract"
            }
            return text, metadata
        except Exception as e:
            logger.error(f"OCR Failed: {e}")
            raise Exception(f"OCR processing failed: {str(e)}")

# Initialize extractors
pdf_extractor = PDFExtractor()
docx_extractor = DOCXExtractor()
pptx_extractor = PPTXExtractor()
csv_extractor = CSVExtractor()
image_extractor = ImageExtractor() # New OCR Extractor

chunker = PassageChunker(chunk_size=512, overlap=50)

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

@app.get("/health")
async def health_check():
    return {"status": "healthy", "service": "ingestion", "ocr_enabled": True}

@app.post("/extract", response_model=ExtractionResponse)
async def extract_document(file: UploadFile = File(...)):
    """
    Extract text and passages from uploaded document (Includes OCR)
    """
    logger.info(f"Received file: {file.filename}, type: {file.content_type}")
    
    try:
        # Read file content
        content = await file.read()
        text = ""
        metadata = {}

        # 1. PDF
        if file.content_type == "application/pdf" or file.filename.endswith(".pdf"):
            text, metadata = pdf_extractor.extract(content)
        
        # 2. Word (DOCX)
        elif file.content_type in ["application/vnd.openxmlformats-officedocument.wordprocessingml.document", 
                                     "application/msword"] or file.filename.endswith((".docx", ".doc")):
            text, metadata = docx_extractor.extract(content)
        
        # 3. PowerPoint (PPTX)
        elif file.content_type in ["application/vnd.openxmlformats-officedocument.presentationml.presentation",
                                     "application/vnd.ms-powerpoint"] or file.filename.endswith((".pptx", ".ppt")):
            text, metadata = pptx_extractor.extract(content)
        
        # 4. CSV
        elif file.content_type == "text/csv" or file.filename.endswith(".csv"):
            text, metadata = csv_extractor.extract(content)
        
        # 5. IMAGES (OCR) - NEW!
        elif file.content_type in ["image/png", "image/jpeg", "image/jpg", "image/webp", "image/tiff"]:
            logger.info("Image detected, running OCR...")
            text, metadata = image_extractor.extract(content)

        # 6. Text Files
        elif file.content_type == "text/plain" or file.filename.endswith(".txt"):
            text = content.decode("utf-8")
            metadata = {"format": "txt"}
        
        else:
            raise HTTPException(status_code=400, detail=f"Unsupported file type: {file.content_type}")
        
        # Validation
        if not text or not text.strip():
            logger.warning(f"No text extracted from {file.filename}")
            text = "[No text found in document]"

        logger.info(f"Extracted {len(text)} characters from {file.filename}")
        
        # Chunk into passages
        passages_data = chunker.chunk(text, metadata)
        
        passages = [
            Passage(
                passage_id=p["passage_id"],
                text=p["text"],
                page=p.get("page"),
                char_start=p["char_start"],
                char_end=p["char_end"],
                metadata=p.get("metadata", {})
            )
            for p in passages_data
        ]
        
        logger.info(f"Created {len(passages)} passages")
        
        return ExtractionResponse(
            filename=file.filename,
            content_type=file.content_type or "unknown",
            total_chars=len(text),
            passages=passages
        )
    
    except Exception as e:
        logger.error(f"Error extracting document: {str(e)}")
        # Print stack trace in logs for debugging
        import traceback
        traceback.print_exc()
        raise HTTPException(status_code=500, detail=f"Extraction failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)