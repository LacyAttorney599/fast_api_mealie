import { useState } from "react";
import styles from "./Courses.module.css";

interface Item {
  qty: string;
  name: string;
  checked: boolean;
}

interface Category {
  name: string;
  items: Item[];
}

const initialCategories: Category[] = [
  {
    name: "Fruits & légumes",
    items: [
      { qty: "3", name: "Poivrons rouges", checked: false },
      { qty: "2", name: "Oignons", checked: false },
      { qty: "6", name: "Pommes de terre", checked: false },
    ],
  },
  {
    name: "Viandes & poissons",
    items: [
      { qty: "500 g", name: "Blancs de poulet", checked: false },
      { qty: "1,2 kg", name: "Épaule de veau", checked: false },
    ],
  },
  {
    name: "Crèmerie",
    items: [
      { qty: "200 g", name: "Crème fraîche", checked: false },
      { qty: "100 g", name: "Beurre", checked: true },
    ],
  },
  {
    name: "Épicerie",
    items: [
      { qty: "400 g", name: "Tomates concassées", checked: false },
      { qty: "2 c. à s.", name: "Huile d'olive", checked: true },
      { qty: "1", name: "Bouquet garni", checked: false },
    ],
  },
];

export default function Courses() {
  const [categories, setCategories] = useState(initialCategories);

  const totalItems = categories.reduce((sum, cat) => sum + cat.items.length, 0);

  function toggleItem(categoryName: string, itemName: string) {
    setCategories((current) =>
      current.map((cat) =>
        cat.name !== categoryName
          ? cat
          : {
              ...cat,
              items: cat.items.map((item) =>
                item.name === itemName ? { ...item, checked: !item.checked } : item,
              ),
            },
      ),
    );
  }

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Liste de courses</h1>
          <p className={styles.subtitle}>Générée depuis le planning du 21 – 27 septembre.</p>
        </div>
        <div className={styles.headerActions}>
          <button className={styles.clearButton}>Effacer les articles cochés</button>
          <button className={styles.addButton}>+ Ajouter un article</button>
        </div>
      </div>

      <div className={styles.body}>
        <div className={styles.categories}>
          {categories.map((cat) => (
            <div key={cat.name} className={styles.category}>
              <span className={styles.categoryName}>{cat.name}</span>
              {cat.items.map((item) => (
                <div key={item.name} className={styles.itemRow}>
                  <input
                    type="checkbox"
                    checked={item.checked}
                    onChange={() => toggleItem(cat.name, item.name)}
                    className={styles.checkbox}
                  />
                  <span className={styles.itemQty}>{item.qty}</span>
                  <span className={item.checked ? styles.itemNameChecked : styles.itemName}>{item.name}</span>
                </div>
              ))}
            </div>
          ))}
        </div>

        <div className={styles.sidebar}>
          <div className={styles.summaryCard}>
            <span className={styles.summaryLabel}>Résumé</span>
            <span className={styles.summaryValue}>{totalItems} articles</span>
            <span className={styles.summaryDetail}>{categories.length} catégories · 2 recettes du planning</span>
          </div>
          <div className={styles.infoCard}>
            <span className={styles.infoTitle}>Fusion automatique</span>
            <span className={styles.infoText}>
              Les quantités identiques sont additionnées automatiquement entre les recettes du planning.
            </span>
          </div>
          <button className={styles.shareButton}>Partager la liste</button>
        </div>
      </div>
    </>
  );
}
