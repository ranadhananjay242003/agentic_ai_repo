FROM python:3.11-slim

WORKDIR /app

# 1. Install System Dependencies (Tesseract OCR and Java 21 for Table Extraction)
RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    libtesseract-dev \
    # Using openjdk-21-jre-headless as suggested by Debian package manager
    openjdk-21-jre-headless \ 
    && rm -rf /var/lib/apt/lists/*

# 2. Install Python Dependencies
COPY requirements.txt .
# Install ghostscript and tk for camelot's dependencies
RUN apt-get update && apt-get install -y ghostscript python3-tk \
    && pip install --no-cache-dir -r requirements.txt \
    && rm -rf /var/lib/apt/lists/*

# 3. Copy Application Code
COPY . .

# 4. Run the service
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8001"]