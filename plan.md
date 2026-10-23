# Plan de développement — refonte du frontend Mealie

2026-09-19 · @Someone

Refonte complète de l'interface de Mealie (design, UX, fonctionnalités additionnelles) tout en gardant Mealie comme moteur de données via son API REST. Ce plan découpe le projet en phases, du cadrage technique jusqu'au déploiement sur le NAS.

Maquettes associées : [canvas de maquettes UI](https://claude.ai/artifact/4M3Le5daUtKQDM6xoCFmJ4).

## Objectifs et périmètre

- Remplacer entièrement le frontend de Mealie par une interface repensée (design, UX), en gardant Mealie comme moteur de données via son API REST (recettes, planificateur, liste de courses, foyer)
- Ajouter des fonctionnalités absentes de Mealie : aperçu de recette retravaillé, enrichissement automatique par photo, imports étendus (photo unique, JSON depuis un livre PDF)
- Fusionner l'ancienne petite appli d'ajout de recettes (formulaire, import photo, import JSON) directement dans ce nouveau frontend plutôt que de la garder à part

## Architecture technique

Depuis le 2026-10-23 (voir « Migration Rust / WebAssembly » plus bas) :

- Frontend : application Rust ([Leptos](https://leptos.dev) 0.8, rendu côté client) compilée en WebAssembly avec Trunk ; elle remplace la SPA React + Vite + TypeScript et reprend ses écrans à l'identique
- Pas de backend applicatif : un reverse proxy Caddy sert l'application et relaie `/api/*` vers Mealie en y ajoutant le token d'API, `/ollama/*` vers Ollama, et sert `/config.json` (modèle Ollama). Le backend-for-frontend FastAPI (`app/`) disparaît ; sa logique (mise en forme des recettes, recettes de saison, liste de courses, planning IA) est reprise dans `src/api/` et s'exécute dans le navigateur
- Dépôt : crate Rust à la racine — `src/` (application), `style/` (CSS), `deploy/` (Caddy), `scripts/fetch-tesseract.sh` (OCR) ; en dev, `trunk serve` (:8080) derrière `deploy/Caddyfile.dev` (:8000) ; en prod, une seule image Docker (compilation WebAssembly, puis Caddy)
- OCR (import photo unique) : Tesseract dans le navigateur (tesseract.js, WebAssembly) avec le modèle français `tessdata_fast`, le même que le paquet Debian `tesseract-ocr-fra` utilisé auparavant ; même prétraitement (niveaux de gris, autocontraste, agrandissement ×2 des petites images) et même reconstitution de l'ordre de lecture des fiches en colonnes
- Structuration (import photo unique) : Ollama + Qwen2.5:3b, déjà déployé et testé sur le NAS, appelé depuis le navigateur via le proxy
- Extraction de livres de recettes (PDF scanné ou natif) : Claude (Sonnet 5 / Opus 5), à qui le PDF est donné directement, retourne un JSON structuré des recettes détectées
- Enrichissement photo (pas encore réalisé) : API de banque d'images (Unsplash ou Pexels) ; sans backend, sa clé devra être ajoutée par Caddy comme le token Mealie
- Données : 100 % dans Mealie via son API REST ; ce nouveau frontend ne conserve pas de base de données propre. Seule exception : les types de service par catégorie (planning IA), un réglage enregistré dans le navigateur

## Découpage en phases

Estimations en soirs/week-ends (travail en dehors des heures de support IT) ; à ajuster selon le temps réellement disponible.

| Phase | Contenu principal | Estimation | Livrable |
| --- | --- | --- | --- |
| 0 — Cadrage technique | Choix du framework (React/Vue), architecture du backend-for-frontend, authentification à l'API Mealie | 3–5 jours | Décisions actées, squelette de dépôt |
| 1 — Fondations | Backend-for-frontend connecté en lecture à l'API Mealie, premier écran Liste avec vraies données | 1–2 semaines | Liste de recettes fonctionnelle |
| 2 — Écrans principaux | Détail recette, planificateur, liste de courses, en lecture/écriture complètes | 2–3 semaines | Parité fonctionnelle avec Mealie |
| 3 — Ajout & import photo | Formulaire manuel, intégration Tesseract + Ollama pour l'import photo unique | 1–2 semaines | Ajout manuel et import photo opérationnels |
| 4 — Import JSON (livres PDF) | Écran de collage/upload JSON, aperçu des recettes détectées, validation, envoi groupé | 3–5 jours | Import de livre de recettes complet |
| 5 — Enrichissement photo | Intégration de l'API banque d'images, sauvegarde du résultat dans Mealie | 3–5 jours | Recettes sans photo enrichies automatiquement |
| 5b — Planning saisonnier | Calendrier légumes/fruits de saison (France), détection des recettes de saison, mise en avant dans le planificateur | 2–3 jours | Recettes de saison visibles au moment de planifier |
| 5c — Génération IA du planning | Bouton "Générer avec l'IA" sur le Planificateur : remplit les créneaux vides à partir des recettes existantes (Ollama), en tenant compte de la saison et de la variété | 3–5 jours | Planning hebdomadaire généré automatiquement, éditable ensuite |
| 6 — Déploiement & finition | Stack Docker Compose définitive, bascule du volume Ollama sur SSD, tests, ajustements UI | 1 semaine | Solution en production sur le NAS |
| 7 — Migration Rust / WebAssembly | Réécriture du frontend en Rust/Leptos, suppression du BFF au profit d'un reverse proxy Caddy, OCR dans le navigateur | — | Une seule image Caddy, plus de backend applicatif |

Total indicatif : 8 à 11 semaines à temps partiel (+ 1 semaine pour 5b/5c).

## Phases 5b/5c — décisions de conception (2026-09-20)

- Calendrier de saison codé en dur côté backend (France, par mois) — pas d'API externe, ces données ne changent pas
- La liste `GET /api/recipes` de Mealie ne contient pas les ingrédients (il faut le détail par recette pour savoir si c'est "de saison"). Décision initiale (2026-09-20) : ne calculer ça qu'à la demande (Planificateur, génération IA), jamais sur l'écran Liste. **Révisée le même jour** suite à une demande explicite d'afficher le badge sur la Liste : le calcul y est maintenant fait, mais protégé par (1) un sémaphore limitant à 6 requêtes Mealie concurrentes — un `asyncio.gather` sans limite a fait planter le calcul par timeout sur un vrai test à 36 recettes, pas juste "lent" — et (2) un cache mémoire d'une heure côté BFF (`app/services/mealie.py::list_recipes_with_season`), qui ramène les appels suivants de ~10s à ~40ms. Depuis la migration Rust (2026-10-23), même limite de 6 requêtes et même cache d'une heure, mais dans le navigateur (`src/api/mealie.rs`)
- La génération IA ne remplit que les créneaux vides de la semaine affichée — n'écrase jamais un créneau déjà planifié
- Le modèle utilisé est le même Ollama/Qwen2.5:3b déjà en place — à valider en pratique : choisir/répartir des recettes dans une liste est une tâche différente (plus proche de la classification) de la structuration de texte libre où ce modèle avait montré des limites en Phase 3

## Décisions actées (Phase 0, 2026-09-20)

- Framework SPA : **React + Vite + TypeScript** (écosystème le plus large, choix par défaut le plus sûr pour ce projet) — remplacé le 2026-10-23 par Rust/Leptos
- Authentification à l'API Mealie : **token d'API**, détenu uniquement côté BFF (`app/config.py` / `.env`, jamais exposé au navigateur) — reprend le fonctionnement déjà en place et testé dans l'ancienne petite appli. Depuis le 2026-10-23, c'est le reverse proxy Caddy qui le détient (variable `MEALIE_API_TOKEN`) ; il n'est toujours jamais exposé au navigateur
- Moment de l'enrichissement photo : **à l'import**, résultat sauvegardé dans Mealie (option recommandée du plan, évite de solliciter l'API de banque d'images à chaque consultation)

Squelette de dépôt posé : `frontend/` scaffoldé (Vite React TS + react-router-dom + @tanstack/react-query), design system extrait des maquettes (`frontend/src/styles/tokens.css`), Sidebar partagée et les 6 écrans portés avec données statiques (fidèles aux maquettes, pas encore branchés à Mealie — c'est l'objet de la Phase 1).

## Migration Rust / WebAssembly — décisions (2026-10-23)

- Mealie détient toutes les données : le backend Python ne servait qu'à cacher le token et à héberger l'OCR et les appels Ollama. Le projet devient une application Rust compilée en WebAssembly, utilisable depuis le téléphone comme avant, sans backend applicatif
- Le reverse proxy Caddy (`deploy/backends.caddy`) remplace le BFF : il ajoute le token Mealie (en-tête `Authorization`), bascule sur `MEALIE_FALLBACK_URL` si Mealie ne répond pas (vide : pas de secours) et retire l'en-tête `Origin` vers Ollama, qui refuse les origines inconnues. L'application et les API sont servies par la même origine : pas de CORS à configurer (Mealie n'en envoie pas)
- Fidélité : l'application Rust reprend les écrans React (mêmes classes CSS, mêmes textes) et la logique Python (prompts identiques, même filtrage des réponses du LLM, même tirage pseudo-aléatoire pour le planning IA — identique au `random` de Python). Les calculs repris de Python et de Pillow sont couverts par des tests unitaires (`cargo test`) avec des valeurs de référence calculées par CPython. Validation de bout en bout le 2026-10-23 : les mêmes scénarios rejoués dans Edge sur l'ancienne et la nouvelle application, face à un Mealie et un Ollama simulés, donnent des résultats identiques sur les 7 écrans (écritures envoyées à Mealie, prompts Ollama, textes affichés, captures d'écran) ; l'import photo (OCR dans le navigateur) retrouve le texte d'une fiche à deux colonnes dans le bon ordre de lecture
- OCR dans le navigateur : tesseract.js et le modèle français sont servis par l'application elle-même (`vendor/tesseract/`, téléchargés par `scripts/fetch-tesseract.sh` avec empreintes sha256 figées), sans CDN. Le moteur WebAssembly peut différer légèrement de la version Debian utilisée auparavant : à valider sur de vraies photos
- Types de service par catégorie : enregistrés dans le navigateur (`localStorage`), avec pour valeurs par défaut celles qui étaient stockées côté serveur (`data/course_types.json`). Conséquence : chaque appareil a ses propres réglages ; un changement fait sur le téléphone n'apparaît pas sur l'ordinateur
- Cache des recettes de saison : propre à chaque navigateur (mémoire + `localStorage`, 1 h, par mois) ; le premier chargement sur un appareil refait le calcul
- Déploiement : même `docker-compose.yml` et mêmes variables d'environnement ; l'image ne contient plus ni Python ni Tesseract, seulement Caddy et les fichiers de l'application

## Risques et points de vigilance

- Pas de GPU sur le NAS : rester sur des modèles Ollama de 1,5 à 3B pour garder une latence raisonnable
- Les clés d'API tierces (banque d'images) ne doivent jamais transiter par le navigateur, uniquement par le reverse proxy
- L'agrégation des quantités dans la liste de courses dépend entièrement de champs Food + Unit correctement remplis à la création — point de vigilance principal des flux d'import automatisés
- Le volume du modèle Ollama doit rester sur le SSD, pas sur le tableau RAID

## Hors périmètre

- Automatisation complète d'une commande drive sur un site marchand : pas d'API publique chez les enseignes françaises, et l'automatisation par script violerait leurs CGU
- Automatisation n8n de l'import PDF/photo : explorée mais mise de côté au profit de la fusion dans le nouveau frontend ; piste possible plus tard pour un traitement en arrière-plan sans intervention
