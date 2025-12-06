#!/bin/bash
# Agentic AI Knowledge Workflow Setup Script (Linux/Mac)
# This script checks prerequisites and sets up the development environment

set -e

echo "========================================"
echo "Agentic AI Setup (Linux/Mac)"
echo "========================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check Rust
echo -e "${YELLOW}[1/5] Checking Rust...${NC}"
if command_exists rustc; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}✓ Rust installed: $RUST_VERSION${NC}"
else
    echo -e "${RED}✗ Rust not found${NC}"
    echo -e "${YELLOW}Installing Rust via rustup...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Rust installed successfully${NC}"
fi

# Check Docker
echo -e "\n${YELLOW}[2/5] Checking Docker...${NC}"
if command_exists docker; then
    DOCKER_VERSION=$(docker --version)
    echo -e "${GREEN}✓ Docker installed: $DOCKER_VERSION${NC}"
    
    # Check if Docker daemon is running
    if docker ps &> /dev/null; then
        echo -e "${GREEN}✓ Docker daemon is running${NC}"
    else
        echo -e "${RED}✗ Docker daemon is not running${NC}"
        echo -e "${YELLOW}Please start Docker and run this script again${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ Docker not found${NC}"
    echo -e "${YELLOW}Please install Docker from: https://docs.docker.com/get-docker/${NC}"
    exit 1
fi

# Check Docker Compose
echo -e "\n${YELLOW}[3/5] Checking Docker Compose...${NC}"
if command_exists docker-compose || docker compose version &> /dev/null; then
    echo -e "${GREEN}✓ Docker Compose available${NC}"
else
    echo -e "${RED}✗ Docker Compose not found${NC}"
    echo -e "${YELLOW}Please install Docker Compose${NC}"
    exit 1
fi

# Check Node.js
echo -e "\n${YELLOW}[4/5] Checking Node.js...${NC}"
if command_exists node; then
    NODE_VERSION=$(node --version)
    echo -e "${GREEN}✓ Node.js installed: $NODE_VERSION${NC}"
else
    echo -e "${YELLOW}⚠ Node.js not found (optional for MVP)${NC}"
fi

# Create .env file
echo -e "\n${YELLOW}[5/5] Setting up environment...${NC}"
if [ ! -f ".env" ]; then
    echo -e "${YELLOW}Creating .env file...${NC}"
    cat > .env << 'EOF'
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
EOF
    echo -e "${GREEN}✓ Created .env file${NC}"
else
    echo -e "${GREEN}✓ .env file already exists${NC}"
fi

# Summary
echo ""
echo -e "${CYAN}========================================${NC}"
echo -e "${GREEN}Setup Complete!${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "${NC}1. Start the services:${NC}"
echo -e "${CYAN}   docker-compose up --build${NC}"
echo ""
echo -e "${NC}2. Wait for all services to be healthy (this may take a few minutes)${NC}"
echo ""
echo -e "${NC}3. Access the services:${NC}"
echo -e "${CYAN}   - Orchestrator API:  http://localhost:8080${NC}"
echo -e "${CYAN}   - Health Check:      http://localhost:8080/health${NC}"
echo -e "${CYAN}   - Metrics:           http://localhost:8080/metrics${NC}"
echo -e "${CYAN}   - Ingestion API:     http://localhost:8001/docs${NC}"
echo -e "${CYAN}   - Embedding API:     http://localhost:8002/docs${NC}"
echo -e "${CYAN}   - Vector DB API:     http://localhost:8003/docs${NC}"
echo ""
echo -e "${NC}4. Test with a sample query:${NC}"
echo -e "${CYAN}   See README.md for API examples${NC}"
echo ""
echo -e "${YELLOW}For troubleshooting, check README.md or run:${NC}"
echo -e "${CYAN}   docker-compose logs -f${NC}"
echo ""
