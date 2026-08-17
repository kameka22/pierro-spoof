# Quick Start

## Première utilisation

### 1. Prérequis

**macOS:**
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js
brew install node

# Xcode Command Line Tools
xcode-select --install
```

**Windows:**
- [Rust](https://rustup.rs)
- [Node.js](https://nodejs.org)
- [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### 2. Lancer en mode développement

```bash
./dev.sh
```

Au premier lancement, cela va :
1. Installer le CLI Tauri (~5-10 minutes, une seule fois)
2. Installer les dépendances npm
3. Compiler et lancer l'application

### 3. Build production

```bash
./build.sh   # macOS/Linux
.\build.ps1  # Windows
```

## Utilisation

1. **Sélectionner un fichier .torrent**
2. **Choisir le client** (qBittorrent 4.0.3 ou 4.3.3)
3. **Configurer les valeurs initiales** :
   - Download initial : `0%`, `50%`, `90%`, `5GB`, etc.
   - Upload initial : `0%`, `1GB`, etc.
4. **Définir les vitesses** :
   - Download : `100` (KB/s par défaut)
   - Upload : `1000` (KB/s par défaut)
5. **Cliquer sur Démarrer**

L'application va :
- Envoyer un announce "started" au tracker
- Simuler le téléchargement/upload selon les vitesses configurées
- Mettre à jour le tracker périodiquement
- Afficher la progression en temps réel

## Dépannage

### Le CLI Tauri ne s'installe pas

Installation manuelle :
```bash
cargo install tauri-cli --version "^2.0.0"
```

### Erreur de compilation

Vérifier que Rust est à jour :
```bash
rustup update
```

### L'interface ne se lance pas

Vérifier que Node.js est installé :
```bash
node --version  # Doit afficher v18+ ou v20+
```

### Port déjà utilisé (dev mode)

Le frontend utilise le port 5173 par défaut. Si occupé :
```bash
# Modifier frontend/vite.config.ts
server: {
  port: 5174  // ou autre port
}
```

## Fonctionnalités avancées

### Ajouter un profil client

1. Créer `crates/core/profiles/nouveau-client.json`
2. Éditer `crates/core/src/emulation/mod.rs`
3. Ajouter le profil dans `load()` et `available_profiles()`
4. Recompiler

### Changer l'icône

Remplacer : `crates/app/icons/icon.png` (512x512 recommandé)

### Debug mode

```bash
cd crates/app
TAURI_DEBUG=1 cargo tauri dev
```

## Ressources

- [Documentation Tauri](https://tauri.app)
- [Rust Book](https://doc.rust-lang.org/book/)
- [React Docs](https://react.dev)
