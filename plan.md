# Plan de développement — refonte du frontend Mealie

2026-09-19 · @Someone

Refonte complète de l'interface de Mealie (design, UX, fonctionnalités additionnelles) tout en gardant Mealie comme moteur de données via son API REST. Ce plan découpe le projet en phases, du cadrage technique jusqu'au déploiement sur le NAS.

Maquettes associées : [canvas de maquettes UI](https://claude.ai/artifact/4M3Le5daUtKQDM6xoCFmJ4).

## Objectifs et périmètre

- Remplacer entièrement le frontend de Mealie par une interface repensée (design, UX), en gardant Mealie comme moteur de données via son API REST (recettes, planificateur, liste de courses, foyer)
- Ajouter des fonctionnalités absentes de Mealie : aperçu de recette retravaillé, enrichissement automatique par photo, imports étendus (photo unique, JSON depuis un livre PDF)
- Fusionner l'ancienne petite appli d'ajout de recettes (formulaire, import photo, import JSON) directement dans ce nouveau frontend plutôt que de la garder à part

## Architecture technique

- Frontend : SPA React + Vite + TypeScript pour une navigation riche — galeries, filtres, vues détaillées fluides
- Backend-for-frontend : nécessaire pour dissimuler les clés d'API tierces (Mealie, banque d'images) et héberger la logique d'import (OCR, appel LLM) ; FastAPI est le candidat naturel — c'est l'app existante (`app/`), dont les routes évolueront vers une API JSON au lieu des templates Jinja2 actuels
- Dépôt : monorepo — `frontend/` (SPA) à côté de `app/` (BFF FastAPI) ; en dev, Vite proxy `/api/*` vers `http://127.0.0.1:8000` (voir `frontend/vite.config.ts`) ; en prod, un même reverse proxy ou FastAPI servira aussi les fichiers buildés du SPA (à affiner en Phase 6)
- OCR (import photo unique) : Tesseract + prétraitement OpenCV
- Structuration (import photo unique) : Ollama + Qwen2.5:3b, déjà déployé et testé sur le NAS
- Extraction de livres de recettes (PDF scanné ou natif) : Claude (Sonnet 5 / Opus 5), à qui le PDF est donné directement, retourne un JSON structuré des recettes détectées
- Enrichissement photo : API de banque d'images (Unsplash ou Pexels), appelée par le backend au moment de l'import
- Données : 100 % dans Mealie via son API REST ; ce nouveau frontend ne conserve pas de base de données propre

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

Total indicatif : 8 à 11 semaines à temps partiel (+ 1 semaine pour 5b/5c).

## Phases 5b/5c — décisions de conception (2026-09-20)

- Calendrier de saison codé en dur côté backend (France, par mois) — pas d'API externe, ces données ne changent pas
- La liste `GET /api/recipes` de Mealie ne contient pas les ingrédients (il faut le détail par recette pour savoir si c'est "de saison") : ce calcul n'est donc fait qu'à la demande (Planificateur, génération IA), jamais sur l'écran Liste à chaque chargement, pour ne pas dégrader les perfs si le livre de recettes grossit beaucoup (import PDF en masse, Phase 4)
- La génération IA ne remplit que les créneaux vides de la semaine affichée — n'écrase jamais un créneau déjà planifié
- Le modèle utilisé est le même Ollama/Qwen2.5:3b déjà en place — à valider en pratique : choisir/répartir des recettes dans une liste est une tâche différente (plus proche de la classification) de la structuration de texte libre où ce modèle avait montré des limites en Phase 3

## Décisions actées (Phase 0, 2026-09-20)

- Framework SPA : **React + Vite + TypeScript** (écosystème le plus large, choix par défaut le plus sûr pour ce projet)
- Authentification à l'API Mealie : **token d'API**, détenu uniquement côté BFF (`app/config.py` / `.env`, jamais exposé au navigateur) — reprend le fonctionnement déjà en place et testé dans l'ancienne petite appli
- Moment de l'enrichissement photo : **à l'import**, résultat sauvegardé dans Mealie (option recommandée du plan, évite de solliciter l'API de banque d'images à chaque consultation)

Squelette de dépôt posé : `frontend/` scaffoldé (Vite React TS + react-router-dom + @tanstack/react-query), design system extrait des maquettes (`frontend/src/styles/tokens.css`), Sidebar partagée et les 6 écrans portés avec données statiques (fidèles aux maquettes, pas encore branchés à Mealie — c'est l'objet de la Phase 1).

## Risques et points de vigilance

- Pas de GPU sur le NAS : rester sur des modèles Ollama de 1,5 à 3B pour garder une latence raisonnable
- Les clés d'API tierces (banque d'images) ne doivent jamais transiter par le navigateur, uniquement par le backend
- L'agrégation des quantités dans la liste de courses dépend entièrement de champs Food + Unit correctement remplis à la création — point de vigilance principal des flux d'import automatisés
- Le volume du modèle Ollama doit rester sur le SSD, pas sur le tableau RAID

## Hors périmètre

- Automatisation complète d'une commande drive sur un site marchand : pas d'API publique chez les enseignes françaises, et l'automatisation par script violerait leurs CGU
- Automatisation n8n de l'import PDF/photo : explorée mais mise de côté au profit de la fusion dans le nouveau frontend ; piste possible plus tard pour un traitement en arrière-plan sans intervention
