import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { fetchRecipes } from "../lib/api";
import styles from "./Liste.module.css";

const filters = ["Tout", "Rapide", "Végétarien", "Dessert", "Plat principal"];

export default function Liste() {
  const [search, setSearch] = useState("");

  const { data: recipes, isLoading, isError } = useQuery({
    queryKey: ["recipes", search],
    queryFn: () => fetchRecipes(search),
  });

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Recettes</h1>
          <p className={styles.subtitle}>
            {isLoading ? "Chargement…" : `${recipes?.length ?? 0} recettes dans votre livre`}
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
        {filters.map((filter, i) => (
          <span key={filter} className={i === 0 ? styles.filterActive : styles.filter}>
            {filter}
          </span>
        ))}
      </div>

      {isError && (
        <p className={styles.error}>
          Impossible de charger les recettes depuis Mealie. Vérifiez que le backend et Mealie sont bien joignables.
        </p>
      )}

      {!isLoading && !isError && recipes?.length === 0 && (
        <p className={styles.empty}>Aucune recette ne correspond à votre recherche.</p>
      )}

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
    </>
  );
}
