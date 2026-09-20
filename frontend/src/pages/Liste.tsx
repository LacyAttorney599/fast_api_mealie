import { Link } from "react-router-dom";
import styles from "./Liste.module.css";

const filters = ["Tout", "Rapide", "Végétarien", "Dessert", "Plat principal"];

const recipes = [
  { name: "Poulet basquaise", time: "45 min", tag: "Plat principal", color: "#E3B9A4", hasPhoto: true },
  { name: "Tarte tatin", time: "1 h 10", tag: "Dessert", color: "#E7C7A6", hasPhoto: true },
  { name: "Risotto aux champignons", time: "35 min", tag: "Végétarien", color: "#C9D1B8", hasPhoto: true },
  { name: "Soupe à l'oignon gratinée", time: "50 min", tag: "Entrée", color: "#D9C9A0", hasPhoto: true },
  { name: "Curry de légumes", time: "30 min", tag: "Végétarien", color: "#B9C9B0", hasPhoto: false },
  { name: "Tiramisu maison", time: "25 min", tag: "Dessert", color: "#E4CDBF", hasPhoto: true },
  { name: "Quiche lorraine", time: "55 min", tag: "Plat principal", color: "#DEC2A4", hasPhoto: true },
  { name: "Salade de lentilles, chèvre chaud", time: "20 min", tag: "Rapide", color: "#C7CDB0", hasPhoto: true },
  { name: "Blanquette de veau", time: "1 h 30", tag: "Plat principal", color: "#E3B9A4", hasPhoto: true },
];

export default function Liste() {
  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Recettes</h1>
          <p className={styles.subtitle}>{recipes.length} recettes dans votre livre</p>
        </div>
        <div className={styles.search}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#8A7F6E" strokeWidth={2}>
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <span className={styles.searchPlaceholder}>Rechercher une recette…</span>
        </div>
      </div>

      <div className={styles.filters}>
        {filters.map((filter, i) => (
          <span key={filter} className={i === 0 ? styles.filterActive : styles.filter}>
            {filter}
          </span>
        ))}
      </div>

      <div className={styles.grid}>
        {recipes.map((recipe) => (
          <div key={recipe.name} className={styles.card}>
            <Link to="/recette/exemple" className={styles.cardImage} style={{ background: recipe.color }}>
              {recipe.hasPhoto ? (
                <svg className={styles.cardIcon} width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#2B2420" strokeWidth={1.3} opacity={0.35}>
                  <path d="M12 2a7 7 0 0 0-7 7c0 3 2 5 2 8h10c0-3 2-5 2-8a7 7 0 0 0-7-7z" />
                  <path d="M9 21h6" />
                </svg>
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
              <Link to="/recette/exemple" className={styles.cardName}>
                {recipe.name}
              </Link>
              <div className={styles.cardMeta}>
                <span>{recipe.time}</span>
                <span>·</span>
                <span>{recipe.tag}</span>
              </div>
              {!recipe.hasPhoto && <button className={styles.suggestButton}>+ Suggérer une photo</button>}
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
