#!/bin/bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║       Pierro Spoof - Release Tool      ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
echo

# Get current version from tauri.conf.json
CURRENT_VERSION=$(grep '"version"' crates/app/tauri.conf.json | head -1 | sed 's/.*"version": *"\([^"]*\)".*/\1/')

echo -e "${BLUE}Current version:${NC} ${YELLOW}v${CURRENT_VERSION}${NC}"

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Calculate next versions
NEXT_PATCH="$MAJOR.$MINOR.$((PATCH + 1))"
NEXT_MINOR="$MAJOR.$((MINOR + 1)).0"
NEXT_MAJOR="$((MAJOR + 1)).0.0"

echo
echo -e "${BLUE}Select version bump:${NC}"
echo -e "  ${GREEN}1)${NC} Patch  → v${NEXT_PATCH} (bug fixes)"
echo -e "  ${GREEN}2)${NC} Minor  → v${NEXT_MINOR} (new features)"
echo -e "  ${GREEN}3)${NC} Major  → v${NEXT_MAJOR} (breaking changes)"
echo -e "  ${GREEN}4)${NC} Custom version"
echo -e "  ${RED}5)${NC} Cancel"
echo

read -p "Choice [1-5]: " CHOICE

case $CHOICE in
    1) NEW_VERSION="$NEXT_PATCH" ;;
    2) NEW_VERSION="$NEXT_MINOR" ;;
    3) NEW_VERSION="$NEXT_MAJOR" ;;
    4)
        read -p "Enter custom version (without 'v'): " NEW_VERSION
        if [[ ! $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo -e "${RED}Invalid version format. Use X.Y.Z${NC}"
            exit 1
        fi
        ;;
    5)
        echo -e "${YELLOW}Cancelled.${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice.${NC}"
        exit 1
        ;;
esac

echo
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${BLUE}Release Summary:${NC}"
echo -e "  Version: ${YELLOW}v${CURRENT_VERSION}${NC} → ${GREEN}v${NEW_VERSION}${NC}"
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo

# Check for uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${YELLOW}Warning: You have uncommitted changes.${NC}"
    git status --short
    echo
fi

read -p "Proceed with release v${NEW_VERSION}? [y/N]: " CONFIRM
if [[ ! $CONFIRM =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Cancelled.${NC}"
    exit 0
fi

echo
echo -e "${CYAN}Updating version in files...${NC}"

# Update tauri.conf.json
sed -i '' "s/\"version\": \"${CURRENT_VERSION}\"/\"version\": \"${NEW_VERSION}\"/" crates/app/tauri.conf.json
echo -e "  ${GREEN}✓${NC} crates/app/tauri.conf.json"

# Update Cargo.toml (root)
sed -i '' "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" Cargo.toml 2>/dev/null || true

# Update crates/app/Cargo.toml
sed -i '' "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" crates/app/Cargo.toml 2>/dev/null || true
echo -e "  ${GREEN}✓${NC} Cargo.toml files"

# Update package.json
sed -i '' "s/\"version\": \"${CURRENT_VERSION}\"/\"version\": \"${NEW_VERSION}\"/" frontend/package.json 2>/dev/null || true
echo -e "  ${GREEN}✓${NC} frontend/package.json"

echo
echo -e "${CYAN}Committing changes...${NC}"
git add -A
git commit -m "Release v${NEW_VERSION}

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

echo
echo -e "${CYAN}Creating tag v${NEW_VERSION}...${NC}"
git tag "v${NEW_VERSION}"

echo
read -p "Push to origin and trigger builds? [y/N]: " PUSH_CONFIRM
if [[ $PUSH_CONFIRM =~ ^[Yy]$ ]]; then
    echo -e "${CYAN}Pushing to origin...${NC}"
    git push origin main
    git push origin "v${NEW_VERSION}"
    
    echo
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║           Release Complete!            ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo
    echo -e "Version ${GREEN}v${NEW_VERSION}${NC} has been released!"
    echo
    echo -e "${BLUE}GitHub Actions will now build:${NC}"
    echo -e "  • Windows installer (MSI/NSIS)"
    echo -e "  • macOS app (DMG)"
    echo
    echo -e "${BLUE}Check progress:${NC}"
    echo -e "  https://github.com/kameka22/pierro-spoof/actions"
    echo
    echo -e "${BLUE}Release page:${NC}"
    echo -e "  https://github.com/kameka22/pierro-spoof/releases/tag/v${NEW_VERSION}"
else
    echo
    echo -e "${YELLOW}Changes committed and tagged locally but not pushed.${NC}"
    echo -e "To push manually:"
    echo -e "  git push origin main"
    echo -e "  git push origin v${NEW_VERSION}"
fi
