import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";
import { fetchRecipe } from "../lib/api";
import styles from "./Detail.module.css";

export default function Detail() {
  const { slug } = useParams<{ slug: string }>();

  const { data: recipe, isLoading, isError } = useQuery({
    queryKey: ["recipe", slug],
    queryFn: () => fetchRecipe(slug!),
    enabled: Boolean(slug),
  });

  if (isLoading) {
    return <p className={styles.status}>Chargement…</p>;
  }

  if (isError || !recipe) {
    return (
      <>
        <Link to="/" className={styles.back}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#6B5F51" strokeWidth={2.2}>
            <path d="m15 18-6-6 6-6" />
          </svg>
          Retour aux recettes
        </Link>
        <p className={styles.status}>Impossible de charger cette recette depuis Mealie.</p>
      </>
    );
  }

  return (
    <>
      <Link to="/" className={styles.back}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#6B5F51" strokeWidth={2.2}>
          <path d="m15 18-6-6 6-6" />
        </svg>
        Retour aux recettes
      </Link>

      <div className={styles.hero}>
        <div className={styles.image}>
          {recipe.image_url ? (
            <img className={styles.photo} src={recipe.image_url} alt="" />
          ) : (
            <svg className={styles.imageIcon} width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#2B2420" strokeWidth={1.2} opacity={0.3}>
              <path d="M12 2a7 7 0 0 0-7 7c0 3 2 5 2 8h10c0-3 2-5 2-8a7 7 0 0 0-7-7z" />
              <path d="M9 21h6" />
            </svg>
          )}
        </div>
        <div className={styles.heroInfo}>
          <div className={styles.heroTop}>
            <h1 className={styles.title}>{recipe.name}</h1>
            <div className={styles.actions}>
              <button className={styles.primaryButton}>Ajouter au planning</button>
              <button className={styles.secondaryButton}>Modifier</button>
            </div>
          </div>
          <div className={styles.meta}>
            {[recipe.time, recipe.servings].filter(Boolean).map((value, i, arr) => (
              <span key={value}>
                {value}
                {i < arr.length - 1 && <span className={styles.metaDot}>·</span>}
              </span>
            ))}
          </div>
          {recipe.tags.length > 0 && (
            <div className={styles.tags}>
              {recipe.tags.map((tag) => (
                <span key={tag} className={styles.tag}>
                  {tag}
                </span>
              ))}
            </div>
          )}
          {recipe.description && <p className={styles.description}>{recipe.description}</p>}
        </div>
      </div>

      <div className={styles.body}>
        <div className={styles.ingredients}>
          <h2 className={styles.sectionTitle}>Ingrédients</h2>
          <div className={styles.ingredientList}>
            {recipe.ingredients.map((ing, i) => (
              <div key={i} className={styles.ingredientRow}>
                <input type="checkbox" className={styles.checkbox} />
                {ing.qty && <span className={styles.ingredientQty}>{ing.qty}</span>}
                <span>{ing.food}</span>
              </div>
            ))}
          </div>
        </div>
        <div className={styles.steps}>
          <h2 className={styles.sectionTitle}>Préparation</h2>
          <div className={styles.stepList}>
            {recipe.steps.map((text, i) => (
              <div key={i} className={styles.stepRow}>
                <span className={styles.stepNumber}>{i + 1}</span>
                <p className={styles.stepText}>{text}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </>
  );
}
