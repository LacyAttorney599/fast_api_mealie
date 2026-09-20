import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { createCookbook, fetchCookbooks, fetchRecipes, fetchSeasonalRecipes } from "../lib/api";
import styles from "./Liste.module.css";

export default function Liste() {
  const [search, setSearch] = useState("");
  const [selectedCookbook, setSelectedCookbook] = useState<string | null>(null);
  const [creatingCookbook, setCreatingCookbook] = useState(false);
  const [newCookbookName, setNewCookbookName] = useState("");
  const queryClient = useQueryClient();

  const { data: cookbooks } = useQuery({
    queryKey: ["cookbooks"],
    queryFn: fetchCookbooks,
  });

  const createCookbookMutation = useMutation({
    mutationFn: (name: string) => createCookbook(name),
    onSuccess: (cookbook) => {
      queryClient.invalidateQueries({ queryKey: ["cookbooks"] });
      setSelectedCookbook(cookbook.slug);
      setNewCookbookName("");
      setCreatingCookbook(false);
    },
  });

  function submitNewCookbook() {
    if (newCookbookName.trim()) {
      createCookbookMutation.mutate(newCookbookName.trim());
    } else {
      setCreatingCookbook(false);
    }
  }

  const { data: recipes, isLoading, isError } = useQuery({
    queryKey: ["recipes", search, selectedCookbook],
    queryFn: () => fetchRecipes(search, selectedCookbook ?? ""),
  });

  // Calculé côté BFF avec un cache d'une heure (un appel détail par recette,
  // la liste Mealie ne renvoie pas les ingrédients) : même mis en cache,
  // inutile de le refaire à chaque frappe dans la recherche.
  const { data: seasonalRecipes } = useQuery({
    queryKey: ["recipes-seasonal"],
    queryFn: fetchSeasonalRecipes,
    staleTime: 10 * 60 * 1000,
  });
  const seasonalSlugs = new Set(seasonalRecipes?.filter((r) => r.in_season).map((r) => r.slug));

  const activeCookbookName = cookbooks?.find((c) => c.slug === selectedCookbook)?.name;

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Recettes</h1>
          <p className={styles.subtitle}>
            {isLoading
              ? "Chargement…"
              : `${recipes?.length ?? 0} recettes${activeCookbookName ? ` dans ${activeCookbookName}` : " dans votre livre"}`}
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
        <button
          className={selectedCookbook === null ? styles.filterActive : styles.filter}
          onClick={() => setSelectedCookbook(null)}
        >
          Tout
        </button>
        {cookbooks?.map((cookbook) => (
          <button
            key={cookbook.slug}
            className={selectedCookbook === cookbook.slug ? styles.filterActive : styles.filter}
            onClick={() => setSelectedCookbook(cookbook.slug)}
          >
            {cookbook.name}
          </button>
        ))}
        {creatingCookbook ? (
          <input
            autoFocus
            className={styles.newCookbookInput}
            placeholder="Nom du livre…"
            value={newCookbookName}
            onChange={(e) => setNewCookbookName(e.target.value)}
            onBlur={submitNewCookbook}
            onKeyDown={(e) => e.key === "Enter" && submitNewCookbook()}
          />
        ) : (
          <button className={styles.filter} onClick={() => setCreatingCookbook(true)}>
            + Nouveau livre
          </button>
        )}
      </div>

      {isError && (
        <p className={styles.error}>
          Impossible de charger les recettes depuis Mealie. Vérifiez que le backend et Mealie sont bien joignables.
        </p>
      )}

      {!isLoading && !isError && recipes?.length === 0 && (
        <p className={styles.empty}>Aucune recette ne correspond à votre recherche.</p>
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
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}
