import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import {
  createCookbook,
  fetchCookbookCategories,
  fetchCookbooks,
  fetchRecipes,
  fetchSeasonalRecipes,
  removeRecipeFromCookbook,
} from "../lib/api";
import styles from "./Liste.module.css";

type Filter = "all" | "seasonal" | { cookbook: string };
type NewCookbookMode = "manual" | "category";

export default function Liste() {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<Filter>("seasonal");
  const [creatingCookbook, setCreatingCookbook] = useState(false);
  const [newCookbookName, setNewCookbookName] = useState("");
  const [newCookbookMode, setNewCookbookMode] = useState<NewCookbookMode>("manual");
  const [newCookbookCategoryId, setNewCookbookCategoryId] = useState("");
  const queryClient = useQueryClient();

  const selectedCookbook = typeof filter === "object" ? filter.cookbook : null;

  const { data: cookbooks } = useQuery({
    queryKey: ["cookbooks"],
    queryFn: fetchCookbooks,
  });

  const { data: categories } = useQuery({
    queryKey: ["cookbook-categories"],
    queryFn: fetchCookbookCategories,
    enabled: creatingCookbook,
  });

  function resetNewCookbookForm() {
    setCreatingCookbook(false);
    setNewCookbookName("");
    setNewCookbookMode("manual");
    setNewCookbookCategoryId("");
  }

  const createCookbookMutation = useMutation({
    mutationFn: () =>
      createCookbook(newCookbookName.trim(), newCookbookMode === "category" ? newCookbookCategoryId : undefined),
    onSuccess: (cookbook) => {
      queryClient.invalidateQueries({ queryKey: ["cookbooks"] });
      setFilter({ cookbook: cookbook.slug });
      resetNewCookbookForm();
    },
  });

  const removeFromCookbookMutation = useMutation({
    mutationFn: (recipeSlug: string) => removeRecipeFromCookbook(selectedCookbook!, recipeSlug),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["recipes", search, selectedCookbook] });
    },
  });

  const activeCookbook = cookbooks?.find((c) => c.slug === selectedCookbook);
  const canRemoveFromBook = Boolean(selectedCookbook) && activeCookbook?.manual === true;

  const { data: allRecipes, isLoading, isError } = useQuery({
    queryKey: ["recipes", search, selectedCookbook],
    queryFn: () => fetchRecipes(search, selectedCookbook ?? ""),
  });

  // Calculé côté BFF avec un cache d'une heure (un appel détail par recette,
  // la liste Mealie ne renvoie pas les ingrédients) : même mis en cache,
  // inutile de le refaire à chaque frappe dans la recherche.
  const { data: seasonalRecipes, isLoading: isLoadingSeasonal } = useQuery({
    queryKey: ["recipes-seasonal"],
    queryFn: fetchSeasonalRecipes,
    staleTime: 10 * 60 * 1000,
  });
  const seasonalSlugs = new Set(seasonalRecipes?.filter((r) => r.in_season).map((r) => r.slug));

  const recipes = filter === "seasonal" ? allRecipes?.filter((r) => seasonalSlugs.has(r.slug)) : allRecipes;

  const subtitleSuffix =
    filter === "seasonal" ? " de saison" : activeCookbook ? ` dans ${activeCookbook.name}` : " dans votre livre";

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Recettes</h1>
          <p className={styles.subtitle}>
            {isLoading || (filter === "seasonal" && isLoadingSeasonal)
              ? "Chargement…"
              : `${recipes?.length ?? 0} recette${(recipes?.length ?? 0) !== 1 ? "s" : ""}${subtitleSuffix}`}
          </p>
        </div>
        <div className={styles.search}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#8A7F6E" strokeWidth={2}>
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <input
            className={styles.searchInput}
            placeholder="Rechercher une recette…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      <div className={styles.filters}>
        <button className={filter === "seasonal" ? styles.filterActive : styles.filter} onClick={() => setFilter("seasonal")}>
          🌱 De saison
        </button>
        <button className={filter === "all" ? styles.filterActive : styles.filter} onClick={() => setFilter("all")}>
          Tout
        </button>
        {cookbooks?.map((cookbook) => (
          <button
            key={cookbook.slug}
            className={selectedCookbook === cookbook.slug ? styles.filterActive : styles.filter}
            onClick={() => setFilter({ cookbook: cookbook.slug })}
          >
            {!cookbook.manual && "🏷 "}
            {cookbook.name}
          </button>
        ))}
        <div className={styles.newCookbookWrapper}>
          {creatingCookbook ? (
            <div
              className={styles.newCookbookPanel}
              onBlur={(e) => {
                if (!e.currentTarget.contains(e.relatedTarget as Node)) {
                  resetNewCookbookForm();
                }
              }}
            >
              <input
                autoFocus
                className={styles.newCookbookInput}
                placeholder="Nom du livre…"
                value={newCookbookName}
                onChange={(e) => setNewCookbookName(e.target.value)}
              />
              <div className={styles.newCookbookModeRow}>
                <label className={styles.newCookbookModeOption}>
                  <input
                    type="radio"
                    checked={newCookbookMode === "manual"}
                    onChange={() => setNewCookbookMode("manual")}
                  />
                  Manuel — j'ajoute les recettes moi-même
                </label>
                <label className={styles.newCookbookModeOption}>
                  <input
                    type="radio"
                    checked={newCookbookMode === "category"}
                    onChange={() => setNewCookbookMode("category")}
                  />
                  Automatique — toutes les recettes d'une catégorie
                </label>
              </div>
              {newCookbookMode === "category" && (
                <select
                  className={styles.newCookbookCategorySelect}
                  value={newCookbookCategoryId}
                  onChange={(e) => setNewCookbookCategoryId(e.target.value)}
                >
                  <option value="" disabled>
                    Choisir une catégorie…
                  </option>
                  {categories?.map((cat) => (
                    <option key={cat.id} value={cat.id}>
                      {cat.name}
                    </option>
                  ))}
                </select>
              )}
              <div className={styles.newCookbookActions}>
                <button className={styles.newCookbookCancel} onClick={resetNewCookbookForm}>
                  Annuler
                </button>
                <button
                  className={styles.newCookbookConfirm}
                  disabled={
                    !newCookbookName.trim() ||
                    (newCookbookMode === "category" && !newCookbookCategoryId) ||
                    createCookbookMutation.isPending
                  }
                  onClick={() => createCookbookMutation.mutate()}
                >
                  {createCookbookMutation.isPending ? "Création…" : "Créer"}
                </button>
              </div>
            </div>
          ) : (
            <button className={styles.filter} onClick={() => setCreatingCookbook(true)}>
              + Nouveau livre
            </button>
          )}
        </div>
      </div>

      {isError && (
        <p className={styles.error}>
          Impossible de charger les recettes depuis Mealie. Vérifiez que le backend et Mealie sont bien joignables.
        </p>
      )}

      {!isLoading && !isError && !(filter === "seasonal" && isLoadingSeasonal) && recipes?.length === 0 && (
        <p className={styles.empty}>
          {filter === "seasonal"
            ? "Aucune recette de saison trouvée pour le moment."
            : "Aucune recette ne correspond à votre recherche."}
        </p>
      )}

      <div className={styles.scrollArea}>
        <div className={styles.grid}>
          {recipes?.map((recipe) => {
            const hasPhoto = Boolean(recipe.image_url);
            return (
              <div key={recipe.slug} className={styles.card}>
                <Link
                  to={`/recette/${recipe.slug}`}
                  className={styles.cardImage}
                  style={{ background: hasPhoto ? undefined : recipe.color }}
                >
                  {hasPhoto ? (
                    <img className={styles.cardPhoto} src={recipe.image_url!} alt="" />
                  ) : (
                    <div className={styles.noPhoto}>
                      <span>Pas de photo</span>
                    </div>
                  )}
                  <button aria-label="Ajouter aux favoris" className={styles.favoriteButton}>
                    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#C1592F" strokeWidth={2}>
                      <path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.6l-1-1a5.5 5.5 0 0 0-7.8 7.8l1 1L12 21l7.8-7.8 1-1a5.5 5.5 0 0 0 0-7.6z" />
                    </svg>
                  </button>
                  {seasonalSlugs.has(recipe.slug) && <span className={styles.seasonBadge}>🌱 De saison</span>}
                </Link>
                <div className={styles.cardBody}>
                  <Link to={`/recette/${recipe.slug}`} className={styles.cardName}>
                    {recipe.name}
                  </Link>
                  <div className={styles.cardMeta}>
                    {recipe.time && (
                      <>
                        <span>{recipe.time}</span>
                        <span>·</span>
                      </>
                    )}
                    <span>{recipe.tag ?? "Sans catégorie"}</span>
                  </div>
                  {!hasPhoto && <button className={styles.suggestButton}>+ Suggérer une photo</button>}
                  {canRemoveFromBook && (
                    <button
                      className={styles.removeFromBookButton}
                      disabled={removeFromCookbookMutation.isPending}
                      onClick={() => removeFromCookbookMutation.mutate(recipe.slug)}
                    >
                      Retirer du livre
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}
