"""
Document Ingestion Service
Extracts text and metadata from various document formats (Text + OCR + Audio)
"""
from fastapi import FastAPI, File, UploadFile, HTTPException
from pydantic import BaseModel
from typing import List, Optional, Dict, Any
import logging
import sys
import io
import os
import requests
from pathlib import Path

# --- IMPORTS ---
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

app = FastAPI(title="Document Ingestion Service", version="1.2.0")

# --- CUSTOM EXTRACTORS ---

class ImageExtractor:
    """Extracts text from images using Tesseract OCR"""
    def extract(self, content: bytes) -> tuple[str, dict]:
        try:
            image = Image.open(io.BytesIO(content))
            text = pytesseract.image_to_string(image)
            metadata = {
                "format": image.format,
                "width": image.width,
                "height": image.height,
                "ocr_engine": "tesseract"
            }
            return text, metadata
        except Exception as e:
            logger.error(f"OCR Failed: {e}")
            raise Exception(f"OCR processing failed: {str(e)}")

class AudioExtractor:
    """Transcribes audio using Groq Whisper API"""
    def extract(self, content: bytes, filename: str) -> tuple[str, dict]:
        api_key = os.getenv("GROQ_API_KEY")
        if not api_key:
            raise Exception("GROQ_API_KEY not found in environment variables")

        try:
            # Groq API Endpoint for Audio
            url = "https://api.groq.com/openai/v1/audio/transcriptions"
            
            headers = {
                "Authorization": f"Bearer {api_key}"
            }
            
            # Prepare file for upload
            files = {
                "file": (filename, content)
            }
            
            data = {
                "model": "whisper-large-v3", # Using Groq's fast Whisper model
                "temperature": "0",
                "response_format": "json"
            }

            logger.info(f"Sending audio {filename} to Groq Whisper...")
            response = requests.post(url, headers=headers, files=files, data=data)
            
            if response.status_code != 200:
                logger.error(f"Groq Whisper Error: {response.text}")
                raise Exception(f"Groq API Error: {response.status_code}")

            result = response.json()
            text = result.get("text", "")
            
            metadata = {
                "format": "audio",
                "model": "whisper-large-v3",
                "provider": "groq"
            }
            
            return text, metadata

        except Exception as e:
            logger.error(f"Audio Transcription Failed: {e}")
            raise Exception(f"Transcription failed: {str(e)}")

# Initialize extractors
pdf_extractor = PDFExtractor()
docx_extractor = DOCXExtractor()
pptx_extractor = PPTXExtractor()
csv_extractor = CSVExtractor()
image_extractor = ImageExtractor()
audio_extractor = AudioExtractor() # NEW!

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
    return {"status": "healthy", "service": "ingestion", "ocr": True, "audio": True}

@app.post("/extract", response_model=ExtractionResponse)
async def extract_document(file: UploadFile = File(...)):
    """
    Extract text from Documents, Images, and Audio
    """
    logger.info(f"Received file: {file.filename}, type: {file.content_type}")
    
    try:
        content = await file.read()
        text = ""
        metadata = {}

        # 1. PDF
        if file.content_type == "application/pdf" or file.filename.endswith(".pdf"):
            text, metadata = pdf_extractor.extract(content)
        
        # 2. Office Docs
        elif file.filename.endswith((".docx", ".doc")):
            text, metadata = docx_extractor.extract(content)
        elif file.filename.endswith((".pptx", ".ppt")):
            text, metadata = pptx_extractor.extract(content)
        
        # 3. CSV
        elif file.filename.endswith(".csv"):
            text, metadata = csv_extractor.extract(content)
        
        # 4. IMAGES (OCR)
        elif file.content_type.startswith("image/") or file.filename.lower().endswith((".png", ".jpg", ".jpeg", ".webp")):
            logger.info("Image detected, running OCR...")
            text, metadata = image_extractor.extract(content)

        # 5. AUDIO (Whisper) - NEW!
        elif file.content_type.startswith("audio/") or file.filename.lower().endswith((".mp3", ".wav", ".m4a", ".ogg")):
            logger.info("Audio detected, running Whisper...")
            text, metadata = audio_extractor.extract(content, file.filename)

        # 6. Text Files
        elif file.content_type == "text/plain" or file.filename.endswith(".txt"):
            text = content.decode("utf-8")
            metadata = {"format": "txt"}
        
        else:
            raise HTTPException(status_code=400, detail=f"Unsupported file type: {file.content_type}")
        
        if not text or not text.strip():
            logger.warning(f"No text extracted from {file.filename}")
            text = "[No text found]"

        logger.info(f"Extracted {len(text)} characters")
        
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
        
        return ExtractionResponse(
            filename=file.filename,
            content_type=file.content_type or "unknown",
            total_chars=len(text),
            passages=passages
        )
    
    except Exception as e:
        logger.error(f"Error extracting document: {str(e)}")
        import traceback
        traceback.print_exc()
        raise HTTPException(status_code=500, detail=f"Extraction failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)