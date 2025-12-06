# Agentic AI Knowledge Workflow Setup Script (Windows PowerShell)
# This script checks prerequisites and sets up the development environment

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Agentic AI Setup (Windows)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Function to check if command exists
function Test-Command {
    param($Command)
    try {
        if (Get-Command $Command -ErrorAction Stop) {
            return $true
        }
    } catch {
        return $false
    }
}

# Check Rust
Write-Host "[1/5] Checking Rust..." -ForegroundColor Yellow
if (Test-Command "rustc") {
    $rustVersion = rustc --version
    Write-Host "OK: Rust installed: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "Rust not found" -ForegroundColor Red
    Write-Host "Installing Rust via rustup..." -ForegroundColor Yellow
    
    # Download and run rustup-init
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
    & "$env:TEMP\rustup-init.exe" -y
    
    # Add to PATH for current session
    $env:Path += ";$($env:USERPROFILE)\.cargo\bin"
    
    Write-Host "OK: Rust installed successfully" -ForegroundColor Green
}

# Check Docker
Write-Host "`n[2/5] Checking Docker..." -ForegroundColor Yellow
if (Test-Command "docker") {
    $dockerVersion = docker --version
    Write-Host "OK: Docker installed: $dockerVersion" -ForegroundColor Green
    
    # Check if Docker daemon is running
    try {
        docker ps | Out-Null
        Write-Host "OK: Docker daemon is running" -ForegroundColor Green
    } catch {
        Write-Host "✗ Docker daemon is not running" -ForegroundColor Red
        Write-Host "Please start Docker Desktop and run this script again" -ForegroundColor Yellow
        exit 1
    }
} else {
    Write-Host "Docker not found" -ForegroundColor Red
    Write-Host "Please install Docker Desktop from: https://www.docker.com/products/docker-desktop/" -ForegroundColor Yellow
    Write-Host "After installation, restart this script" -ForegroundColor Yellow
    exit 1
}

# Check Node.js
Write-Host "`n[3/5] Checking Node.js..." -ForegroundColor Yellow
if (Test-Command "node") {
    $nodeVersion = node --version
    Write-Host "OK: Node.js installed: $nodeVersion" -ForegroundColor Green
} else {
    Write-Host "Node.js not found (optional for MVP)" -ForegroundColor Yellow
}

# Create .env file
Write-Host "`n[4/5] Setting up environment..." -ForegroundColor Yellow
if (!(Test-Path ".env")) {
    Write-Host "Creating .env file..." -ForegroundColor Yellow
    @"
# Orchestrator Configuration
PORT=8080
DATABASE_URL=postgres://postgres:postgres@localhost:5432/agentic_ai
REDIS_URL=redis://localhost:6379
LOG_LEVEL=info
RUST_LOG=info

# Service URLs
INGESTION_SERVICE_URL=http://localhost:8001
EMBEDDING_SERVICE_URL=http://localhost:8002
VECTOR_DB_SERVICE_URL=http://localhost:8003

# Security (CHANGE IN PRODUCTION!)
JWT_SECRET=dev-secret-change-in-production

# Optional: OpenAI API Key
# OPENAI_API_KEY=sk-...
"@ | Out-File -FilePath ".env" -Encoding UTF8
    
    Write-Host "OK: Created .env file" -ForegroundColor Green
} else {
    Write-Host ".env file already exists" -ForegroundColor Green
}

# Build Rust orchestrator
Write-Host "`n[5/5] Building Rust orchestrator..." -ForegroundColor Yellow
try {
    cargo build --release
    Write-Host "OK: Rust build successful" -ForegroundColor Green
} catch {
    Write-Host "Rust build failed (this is OK for Docker-only setup)" -ForegroundColor Yellow
}

# Summary
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Setup Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Start the services:" -ForegroundColor White
Write-Host "   docker-compose up --build" -ForegroundColor Cyan
Write-Host ""
Write-Host "2. Wait for all services to be healthy (this may take a few minutes)" -ForegroundColor White
Write-Host ""
Write-Host "3. Access the services:" -ForegroundColor White
Write-Host "   - Orchestrator API:  http://localhost:8080" -ForegroundColor Cyan
Write-Host "   - Health Check:      http://localhost:8080/health" -ForegroundColor Cyan
Write-Host "   - Metrics:           http://localhost:8080/metrics" -ForegroundColor Cyan
Write-Host "   - Ingestion API:     http://localhost:8001/docs" -ForegroundColor Cyan
Write-Host "   - Embedding API:     http://localhost:8002/docs" -ForegroundColor Cyan
Write-Host "   - Vector DB API:     http://localhost:8003/docs" -ForegroundColor Cyan
Write-Host ""
Write-Host "4. Test with a sample query:" -ForegroundColor White
Write-Host "   See README.md for API examples" -ForegroundColor Cyan
Write-Host ""
Write-Host "For troubleshooting, check README.md or run:" -ForegroundColor Yellow
Write-Host "   docker-compose logs -f" -ForegroundColor Cyan
Write-Host ""
