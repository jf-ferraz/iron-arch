#!/usr/bin/env bash
# Setup Immich photo management with Docker Compose
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

IMMICH_DIR="${HOME}/immich"
DOCKER_COMPOSE_URL="https://github.com/immich-app/immich/releases/latest/download/docker-compose.yml"
ENV_EXAMPLE_URL="https://github.com/immich-app/immich/releases/latest/download/example.env"

echo -e "${BLUE}Setting up Immich photo management...${NC}"
echo ""

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
  echo -e "${YELLOW}Warning: Docker daemon is not running${NC}"
  echo "Please start Docker with: sudo systemctl start docker"
  echo "To enable Docker on boot: sudo systemctl enable docker"
  echo ""
  read -p "Would you like to start and enable Docker now? [y/N] " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    sudo systemctl start docker
    sudo systemctl enable docker
    echo -e "${GREEN}✓ Docker started and enabled${NC}"
  else
    echo -e "${YELLOW}Skipping Docker setup. You'll need to start it manually later.${NC}"
  fi
  echo ""
fi

# Add user to docker group if not already
if ! groups | grep -q docker; then
  echo -e "${YELLOW}Adding current user to docker group...${NC}"
  sudo usermod -aG docker "$USER"
  echo -e "${GREEN}✓ User added to docker group${NC}"
  echo -e "${YELLOW}Note: You may need to log out and back in for group changes to take effect${NC}"
  echo ""
fi

# Check if Immich directory already exists
if [ -d "$IMMICH_DIR" ]; then
  echo -e "${YELLOW}Immich directory already exists: $IMMICH_DIR${NC}"
  read -p "Would you like to reconfigure it? [y/N] " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${BLUE}Skipping Immich setup${NC}"
    exit 0
  fi
else
  echo -e "${BLUE}Creating Immich directory: $IMMICH_DIR${NC}"
  mkdir -p "$IMMICH_DIR"
fi

cd "$IMMICH_DIR"

# Download docker-compose.yml
echo -e "${BLUE}Downloading docker-compose.yml...${NC}"
if curl -fsSL -o docker-compose.yml "$DOCKER_COMPOSE_URL"; then
  echo -e "${GREEN}✓ docker-compose.yml downloaded${NC}"
else
  echo -e "${RED}Error: Failed to download docker-compose.yml${NC}" >&2
  exit 1
fi

# Download and setup .env file
if [ ! -f ".env" ]; then
  echo -e "${BLUE}Downloading example environment file...${NC}"
  if curl -fsSL -o .env "$ENV_EXAMPLE_URL"; then
    echo -e "${GREEN}✓ .env file downloaded${NC}"

    # Generate a random database password
    DB_PASSWORD=$(openssl rand -base64 32 | tr -dc 'A-Za-z0-9' | head -c 32)

    # Update .env with sensible defaults
    echo -e "${BLUE}Configuring environment variables...${NC}"
    sed -i "s|^UPLOAD_LOCATION=.*|UPLOAD_LOCATION=${IMMICH_DIR}/library|" .env
    sed -i "s|^DB_DATA_LOCATION=.*|DB_DATA_LOCATION=${IMMICH_DIR}/postgres|" .env
    sed -i "s|^DB_PASSWORD=.*|DB_PASSWORD=${DB_PASSWORD}|" .env

    # Try to detect timezone
    if [ -f /etc/timezone ]; then
      TZ=$(cat /etc/timezone)
      sed -i "s|^TZ=.*|TZ=${TZ}|" .env
    elif [ -L /etc/localtime ]; then
      TZ=$(readlink /etc/localtime | sed 's|/usr/share/zoneinfo/||')
      sed -i "s|^TZ=.*|TZ=${TZ}|" .env
    fi

    echo -e "${GREEN}✓ Environment configured${NC}"
  else
    echo -e "${RED}Error: Failed to download example.env${NC}" >&2
    exit 1
  fi
else
  echo -e "${YELLOW}.env file already exists, skipping configuration${NC}"
fi

# Create data directories
mkdir -p "${IMMICH_DIR}/library"
mkdir -p "${IMMICH_DIR}/postgres"

echo ""
echo -e "${GREEN}✓ Immich setup complete!${NC}"
echo ""
echo -e "${BLUE}Installation directory:${NC} $IMMICH_DIR"
echo -e "${BLUE}Upload location:${NC} ${IMMICH_DIR}/library"
echo -e "${BLUE}Database location:${NC} ${IMMICH_DIR}/postgres"
echo ""
echo -e "${BLUE}To start Immich:${NC}"
echo "  cd $IMMICH_DIR"
echo "  docker compose up -d"
echo ""
echo -e "${BLUE}To stop Immich:${NC}"
echo "  cd $IMMICH_DIR"
echo "  docker compose down"
echo ""
echo -e "${BLUE}To view logs:${NC}"
echo "  cd $IMMICH_DIR"
echo "  docker compose logs -f"
echo ""
echo -e "${BLUE}Access Immich:${NC}"
echo "  Web interface: http://localhost:2283"
echo ""
echo -e "${YELLOW}Note:${NC} Review and customize $IMMICH_DIR/.env as needed"
echo ""

# Ask if user wants to start Immich now
read -p "Would you like to start Immich now? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo -e "${BLUE}Starting Immich containers...${NC}"
  docker compose up -d
  echo ""
  echo -e "${GREEN}✓ Immich is now running!${NC}"
  echo -e "${BLUE}Access it at:${NC} http://localhost:2283"
  echo ""
fi
