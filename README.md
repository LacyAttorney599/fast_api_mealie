# Ma Cuisine

Interface web au-dessus de [Mealie](https://mealie.io) : recettes, planning de la semaine, liste de courses,
import de recettes (photo, JSON d'un livre PDF) et planning généré par IA. Mealie reste l'unique source de
données ; l'application n'a pas de base ni de backend à elle.

## Architecture

L'application est écrite en Rust ([Leptos](https://leptos.dev)) et compilée en WebAssembly : tout tourne dans
le navigateur. Un reverse proxy Caddy la sert et relaie les appels :

```
navigateur ──► Caddy :8000 ─┬─ /          → application (fichiers statiques, dist/)
                            ├─ /api/…     → Mealie, avec le token d'API ajouté par Caddy
                            ├─ /ollama/…  → Ollama (structuration des photos, planning IA)
                            └─ /config.json → réglages (modèle Ollama)
```

Le token Mealie n'est donc jamais exposé au navigateur. L'OCR des photos se fait dans le navigateur
(tesseract.js, avec le même modèle français que le paquet Debian `tesseract-ocr-fra`), puis le texte est
structuré par Ollama. Les types de service par catégorie (pour le planning IA) sont enregistrés dans le
navigateur.

| Dossier | Contenu |
| --- | --- |
| `src/api/` | Accès aux données : appels Mealie et Ollama, mises en forme, cache des recettes de saison, OCR |
| `src/pages/`, `src/components/` | Écrans et mise en page |
| `src/util/` | Dates, lecture du JSON d'import, sémantique JavaScript reprise de l'ancienne interface |
| `style/` | Feuilles de style (classes préfixées par écran) |
| `deploy/` | Configuration Caddy (production et développement) |
| `scripts/fetch-tesseract.sh` | Téléchargement de tesseract.js et du modèle français (versions figées) |

## Déploiement

`docker compose up -d --build` construit l'image (compilation WebAssembly, puis image Caddy) et l'expose sur le
port 8000. Variables d'environnement :

| Variable | Rôle | Défaut |
| --- | --- | --- |
| `MEALIE_BASE_URL` | URL de Mealie (ex. `http://192.168.1.250:9925`) | — |
| `MEALIE_FALLBACK_URL` | URL de secours si la première ne répond pas (vide : pas de secours) | `http://172.25.0.1:9000` |
| `MEALIE_API_TOKEN` | Token d'API Mealie | — |
| `OLLAMA_BASE_URL` | URL d'Ollama (ex. `http://192.168.1.250:11434`) | — |
| `OLLAMA_MODEL` | Modèle utilisé | `qwen2.5:3b` |

## Développement

Prérequis : Rust stable avec la cible `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`),
[Trunk](https://trunkrs.dev) et [Caddy](https://caddyserver.com).

```sh
sh scripts/fetch-tesseract.sh                          # une fois : tesseract.js + modèle français
trunk serve                                            # application sur :8080, rechargement automatique
caddy run --config deploy/Caddyfile.dev --envfile .env # proxy sur :8000 (Mealie, Ollama, application)
```

Puis ouvrir http://localhost:8000. Le fichier `.env` reprend les variables du tableau ci-dessus (voir
`.env.example`).

Tests : `cargo test` (logique de l'application, exécutée hors navigateur).
