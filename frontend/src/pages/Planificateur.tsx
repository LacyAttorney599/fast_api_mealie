import { Link } from "react-router-dom";
import styles from "./Planificateur.module.css";

const days = [
  { label: "Lun 21", lunch: null, dinner: "Poulet basquaise" },
  { label: "Mar 22", lunch: "Salade de lentilles", dinner: "Risotto aux champignons" },
  { label: "Mer 23", lunch: null, dinner: "Curry de légumes" },
  { label: "Jeu 24", lunch: "Curry (restes)", dinner: "Quiche lorraine" },
  { label: "Ven 25", lunch: null, dinner: "Blanquette de veau" },
  { label: "Sam 26", lunch: null, dinner: null },
  { label: "Dim 27", lunch: "Tiramisu maison", dinner: "Soupe à l'oignon gratinée" },
];

function MealSlot({ name, dotColor }: { name: string | null; dotColor: string }) {
  if (!name) {
    return <div className={styles.emptySlot}>+ Ajouter</div>;
  }
  return (
    <div className={styles.filledSlot}>
      <span className={styles.dot} style={{ background: dotColor }} />
      <span className={styles.slotName}>{name}</span>
    </div>
  );
}

export default function Planificateur() {
  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Planificateur</h1>
          <p className={styles.subtitle}>Organisez déjeuners et dîners de la semaine.</p>
        </div>
        <div className={styles.weekNav}>
          <button aria-label="Semaine précédente" className={styles.navButton}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#4A4238" strokeWidth={2.2}>
              <path d="m15 18-6-6 6-6" />
            </svg>
          </button>
          <span className={styles.weekLabel}>21 – 27 septembre 2026</span>
          <button aria-label="Semaine suivante" className={styles.navButton}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#4A4238" strokeWidth={2.2}>
              <path d="m9 18 6-6-6-6" />
            </svg>
          </button>
        </div>
      </div>

      <div className={styles.grid}>
        <div />
        {days.map((d) => (
          <div key={d.label} className={styles.dayLabel}>
            {d.label}
          </div>
        ))}

        <div className={styles.rowLabel}>Déjeuner</div>
        {days.map((d) => (
          <MealSlot key={`lunch-${d.label}`} name={d.lunch} dotColor="#C1592F" />
        ))}

        <div className={styles.rowLabel}>Dîner</div>
        {days.map((d) => (
          <MealSlot key={`dinner-${d.label}`} name={d.dinner} dotColor="#6B7A5E" />
        ))}
      </div>

      <div className={styles.footer}>
        <Link to="/courses" className={styles.generateButton}>
          Générer la liste de courses
        </Link>
      </div>
    </>
  );
}
