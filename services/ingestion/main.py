"""
Document Ingestion Service
Extracts text and metadata from various document formats (Robust Version)
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

# --- IMPORTS ---
import camelot
import pandas as pd
import pytesseract
from PIL import Image

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

from extractors import PDFExtractor, DOCXExtractor, PPTXExtractor, CSVExtractor
from chunker import PassageChunker

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(title="Document Ingestion Service", version="1.7.0")

# --- ROBUST EXTRACTOR CLASSES ---

class ImageExtractor:
    """
    Uses Tesseract to read text, then Llama 3.3 (Text) to clean/format it.
    This avoids Vision API dependency issues.
    """
    def extract(self, content: bytes, filename: str) -> tuple[str, dict]:
        text_output = ""
        
        # 1. Run Tesseract OCR (Reliable text extraction)
        try:
            image = Image.open(io.BytesIO(content))
            raw_ocr_text = pytesseract.image_to_string(image)
            logger.info(f"Raw OCR Output length: {len(raw_ocr_text)}")
        except Exception as e:
            logger.error(f"OCR Failed: {e}")
            return "[Image OCR Failed]", {"format": "image", "error": str(e)}

        if not raw_ocr_text.strip():
            return "[No text found in image]", {"format": "image"}

        # 2. Use Llama 3.3 to Structure the Data
        cleaned_text = self.clean_ocr_with_llm(raw_ocr_text)
        
        if cleaned_text:
            text_output = f"--- SMART OCR ANALYSIS ---\n{cleaned_text}\n\n--- RAW SCAN ---\n{raw_ocr_text}"
        else:
            text_output = f"--- RAW SCAN ---\n{raw_ocr_text}"

        metadata = {
            "format": "image",
            "method": "ocr_plus_llm",
            "filename": filename
        }
        return text_output, metadata

    def clean_ocr_with_llm(self, messy_text: str) -> str:
        api_key = os.getenv("GROQ_API_KEY")
        if not api_key: 
            logger.warning("GROQ_API_KEY missing, skipping cleanup")
            return ""

        try:
            url = "https://api.groq.com/openai/v1/chat/completions"
            headers = {
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json"
            }
            
            # Using the stable Text model
            payload = {
                "model": "llama-3.3-70b-versatile", 
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a Data Formatting Assistant. I will provide raw, messy text scanned from an image/chart. Your job is to format it cleanly. If it's a chart, list data as 'Label: Value'. If it's text, correct typos. Output ONLY the cleaned text."
                    },
                    {
                        "role": "user",
                        "content": f"Raw Text:\n{messy_text}"
                    }
                ],
                "temperature": 0.1,
                "max_tokens": 1000
            }

            response = requests.post(url, headers=headers, json=payload)
            if response.status_code == 200:
                return response.json()['choices'][0]['message']['content']
            else:
                logger.error(f"LLM Cleanup Error: {response.text}")
                return ""
        except Exception as e:
            logger.error(f"LLM Cleanup Failed: {e}")
            return ""

class AudioExtractor:
    def extract(self, content: bytes, filename: str) -> tuple[str, dict]:
        api_key = os.getenv("GROQ_API_KEY")
        if not api_key: raise Exception("GROQ_API_KEY not found in environment")
        
        try:
            url = "https://api.groq.com/openai/v1/audio/transcriptions"
            headers = {"Authorization": f"Bearer {api_key}"}
            # Filename is required for Groq to know file type
            files = {"file": (filename, content)} 
            data = {"model": "whisper-large-v3"}
            
            logger.info(f"Sending audio {filename} to Whisper...")
            response = requests.post(url, headers=headers, files=files, data=data)
            
            if response.status_code != 200:
                raise Exception(f"Groq API Error: {response.status_code} - {response.text}")
            
            return response.json().get("text", ""), {"format": "audio", "model": "whisper"}
        except Exception as e:
            raise Exception(f"Transcription failed: {str(e)}")

class PDFTableExtractor:
    def extract_tables_to_text(self, pdf_bytes: bytes) -> str:
        try:
            temp_path = Path("/tmp") / f"temp_{os.getpid()}.pdf"
            with open(temp_path, "wb") as f: f.write(pdf_bytes)
            tables = camelot.read_pdf(str(temp_path), flavor='lattice', pages='all')
            structured_text = []
            if tables.n > 0:
                for i, table in enumerate(tables):
                    df: pd.DataFrame = table.df
                    table_markdown = df.to_markdown(index=False)
                    structured_text.append(f"\n--- TABLE {i+1} START ---\n{table_markdown}\n--- TABLE {i+1} END ---\n")
                return "\n".join(structured_text)
            return ""
        except Exception as e:
            logger.error(f"Camelot Table Extraction Failed: {e}")
            return ""
        finally:
            if temp_path.exists(): os.remove(temp_path)

# Initialize extractors
pdf_extractor = PDFExtractor()
docx_extractor = DOCXExtractor()
pptx_extractor = PPTXExtractor()
csv_extractor = CSVExtractor()
image_extractor = ImageExtractor()
audio_extractor = AudioExtractor() 
table_extractor = PDFTableExtractor()

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

@app.get("/health")
async def health_check():
    return {"status": "healthy", "service": "ingestion"}

@app.post("/extract", response_model=ExtractionResponse)
async def extract_document(file: UploadFile = File(...)):
    try:
        content = await file.read()
        text = ""
        metadata = {}

        if file.content_type == "application/pdf" or file.filename.endswith(".pdf"):
            base_text, metadata = pdf_extractor.extract(content)
            table_text = table_extractor.extract_tables_to_text(content)
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
        
        if not text or not text.strip(): text = "[No text found]"

        passages_data = chunker.chunk(text, metadata)
        passages = [Passage(passage_id=p["passage_id"], text=p["text"], page=p.get("page"), char_start=p["char_start"], char_end=p["char_end"], metadata=p.get("metadata", {})) for p in passages_data]
        
        return ExtractionResponse(filename=file.filename, content_type=file.content_type or "unknown", total_chars=len(text), passages=passages)
        
    except Exception as e:
        logger.error(f"Error: {str(e)}")
        import traceback
        traceback.print_exc()
        raise HTTPException(status_code=500, detail=f"Extraction failed: {str(e)}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)