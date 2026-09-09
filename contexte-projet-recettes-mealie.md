# Contexte projet : mini-appli d'ajout de recettes vers Mealie

## Objectif

Construire une petite application web self-hosted qui simplifie l'ajout de recettes dans Mealie (gestionnaire de recettes déjà déployé), avec deux modes de saisie :
1. Formulaire manuel structuré (ingrédients en quantité / unité / aliment séparés)
2. Import par **photo** d'une recette papier : OCR + LLM local structurent automatiquement le texte en JSON avant de pré-remplir le formulaire pour validation humaine

Le but final : réduire la friction de saisie tout en garantissant des ingrédients bien structurés dans Mealie, ce qui permet à sa liste de courses de fusionner correctement les quantités entre plusieurs recettes.

## Environnement / infra

- NAS ZimaOS, stockage en RAID + un SSD séparé pour les services système/conteneurs
- CPU : Ryzen 5 1600 (6 cœurs / 12 threads, 3.2 GHz, 19 Mo cache), pas de GPU
- 16 Go de RAM
- Orchestration via Docker / Portainer (stacks séparés par service)
- Instance Mealie déjà en place et utilisée au quotidien, avec API REST activée

## Stack technique retenue

- **Backend** : FastAPI (Python)
- **Frontend** : HTMX + Jinja2 (rendu serveur, pas de build JS) — choisi pour rester dans un seul langage et un déploiement Docker minimal, cohérent avec le reste de l'infra self-hosted
- **OCR** : Tesseract (pytesseract), avec un pré-traitement d'image (recadrage, contraste, correction de perspective) avant extraction
- **LLM de structuration** : Ollama + **Qwen2.5:3b**, en conteneur Docker sur le NAS — déjà installé et testé avec succès
- **Cible finale** : API REST de Mealie (`/api/recipes`, ingrédients avec food/unit/quantité structurés)

## Architecture du flux (import photo)

1. Upload d'une photo depuis le téléphone (PWA, `<input type="file" accept="image/*" capture="environment">`)
2. `POST /import/photo` côté FastAPI
3. Tesseract extrait le texte brut de l'image
4. Le texte brut est envoyé à Ollama (`qwen2.5:3b`) avec un prompt de structuration demandant un JSON strict : `{nom, ingredients: [{quantite, unite, aliment}], etapes}`
5. Le JSON pré-remplit le formulaire d'ajout (même écran que la saisie manuelle)
6. L'utilisateur vérifie/corrige les champs
7. Envoi vers l'API Mealie au clic sur "Envoyer à Mealie"

## UI prévue (3 écrans / onglets)

- **Liste** : recherche + filtre sur les recettes existantes, bouton "Nouvelle recette"
- **Ajouter** : formulaire manuel — nom, import par URL, ingrédients en champs séparés (quantité/unité/aliment), étapes en texte libre
- **Photo** : capture/galerie → statut de traitement (Tesseract → Ollama) → redirige vers le formulaire "Ajouter" pré-rempli

## État d'avancement

- [x] Conteneur Ollama déployé sur le NAS (Docker/Portainer), fonctionnel
- [x] Modèle `qwen2.5:3b` téléchargé et testé en ligne de commande (`ollama run`) — réponses correctes, latence jugée acceptable sur ce CPU
- [x] Intégration Tesseract (dépendance dans l'image FastAPI, `app/services/ocr.py`)
- [x] Squelette FastAPI : endpoint recevant une image → Tesseract → Ollama → retour du JSON brut (`app/routers/photo.py`, `app/services/llm.py`)
- [x] Prompt de structuration testé et affiné sur une vraie photo (fiche recette 2 colonnes) — complet et fiable pour un usage courant, quelques cas limites connus (voir mémoire projet)
- [x] UI (les 3 écrans ci-dessus) — design repris du thème Mealie réel (couleurs, police Roboto, cartes/boutons Material Design)
- [x] Connexion à l'API Mealie pour l'envoi final de la recette structurée — testée de bout en bout (formulaire manuel ET flux photo complet) contre l'instance réelle, ingrédients/unités correctement structurés
- [x] Stack Docker Compose définitive réunissant FastAPI + Ollama sur un réseau Docker partagé (`docker-compose.yml`)

## Points d'attention / contraintes

- Le modèle Ollama ne nécessite **aucune sauvegarde** (reproductible via `ollama pull`) ; le volume doit être placé sur le **SSD**, pas sur le tableau RAID, pour la vitesse de chargement et pour ne pas gaspiller la redondance RAID sur des données non critiques
- Pas de GPU sur le NAS : rester sur des modèles Ollama de 1,5B à 3B pour garder une latence raisonnable (le 3B tourne correctement sur ce CPU avec 16 Go de RAM disponibles)
- Pour que FastAPI et Ollama communiquent par nom de service, ils doivent être sur le **même réseau Docker** — soit un réseau externe créé au préalable dans Portainer, soit (préférable) un unique `docker-compose.yml` réunissant les deux services, qui créera le réseau automatiquement
- L'agrégation des quantités dans la liste de courses Mealie dépend entièrement de champs **Food + Unit** correctement remplis à la création de la recette — c'est le point de vigilance principal du prompt de structuration LLM et du formulaire

## Hors périmètre (pour l'instant)

- Automatisation complète d'une commande drive sur un site marchand : pas d'API publique chez les enseignes françaises, et l'automatisation par script violerait leurs CGU. Piste possible mais non prioritaire : extension navigateur qui pré-remplit la recherche produit à partir de la liste de courses Mealie, en laissant la validation panier manuelle.
