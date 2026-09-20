import { Link } from "react-router-dom";
import styles from "./Detail.module.css";

const ingredients = [
  { qty: "500 g", food: "Blancs de poulet" },
  { qty: "2", food: "Poivrons rouges" },
  { qty: "1", food: "Poivron vert" },
  { qty: "1", food: "Oignon" },
  { qty: "400 g", food: "Tomates concassées" },
  { qty: "2 c. à s.", food: "Huile d'olive" },
  { qty: "1 pincée", food: "Piment d'Espelette" },
];

const steps = [
  "Émincer les poivrons et l'oignon, couper le poulet en morceaux.",
  "Faire dorer le poulet à l'huile d'olive dans une cocotte, réserver.",
  "Faire suer les oignons et poivrons 5 minutes dans la même cocotte.",
  "Ajouter les tomates concassées, le piment d'Espelette, sel et poivre.",
  "Remettre le poulet, couvrir et laisser mijoter 25 minutes à feu doux.",
  "Rectifier l'assaisonnement et servir bien chaud avec du riz.",
];

export default function Detail() {
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
          <svg className={styles.imageIcon} width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#2B2420" strokeWidth={1.2} opacity={0.3}>
            <path d="M12 2a7 7 0 0 0-7 7c0 3 2 5 2 8h10c0-3 2-5 2-8a7 7 0 0 0-7-7z" />
            <path d="M9 21h6" />
          </svg>
          <span className={styles.suggestedBadge}>
            Photo suggérée automatiquement
            <span className={styles.changeLink}>Changer</span>
          </span>
        </div>
        <div className={styles.heroInfo}>
          <div className={styles.heroTop}>
            <h1 className={styles.title}>Poulet basquaise</h1>
            <div className={styles.actions}>
              <button className={styles.primaryButton}>Ajouter au planning</button>
              <button className={styles.secondaryButton}>Modifier</button>
            </div>
          </div>
          <div className={styles.meta}>
            <span>45 min</span>
            <span>·</span>
            <span>4 personnes</span>
            <span>·</span>
            <span>Facile</span>
          </div>
          <div className={styles.tags}>
            <span className={styles.tag}>Plat principal</span>
            <span className={styles.tag}>Sans gluten</span>
            <span className={styles.tag}>Basque</span>
          </div>
          <p className={styles.description}>
            Un classique du Sud-Ouest : blancs de poulet mijotés avec poivrons, tomates et oignons, parfumés à l'ail et
            au piment d'Espelette.
          </p>
        </div>
      </div>

      <div className={styles.body}>
        <div className={styles.ingredients}>
          <h2 className={styles.sectionTitle}>Ingrédients</h2>
          <div className={styles.ingredientList}>
            {ingredients.map((ing) => (
              <div key={ing.food} className={styles.ingredientRow}>
                <input type="checkbox" className={styles.checkbox} />
                <span className={styles.ingredientQty}>{ing.qty}</span>
                <span>{ing.food}</span>
              </div>
            ))}
          </div>
        </div>
        <div className={styles.steps}>
          <h2 className={styles.sectionTitle}>Préparation</h2>
          <div className={styles.stepList}>
            {steps.map((text, i) => (
              <div key={text} className={styles.stepRow}>
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
