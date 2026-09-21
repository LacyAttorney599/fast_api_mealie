import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import {
  addShoppingItem,
  clearAllShoppingItems,
  clearCheckedShoppingItems,
  fetchShoppingList,
  generateShoppingList,
  toggleShoppingItem,
} from "../lib/api";
import { formatWeekRange, getMonday, toISODate } from "../lib/dates";
import styles from "./Courses.module.css";

const SHOPPING_LIST_KEY = ["shoppinglist"];

export default function Courses() {
  const [searchParams] = useSearchParams();
  const [addingItem, setAddingItem] = useState(false);
  const [newItemText, setNewItemText] = useState("");
  const queryClient = useQueryClient();

  const monday = getMonday(new Date());
  const start = searchParams.get("start") ?? toISODate(monday);
  const end = searchParams.get("end") ?? toISODate(monday);
  const weekLabel = formatWeekRange(new Date(start));

  const { data: list, isLoading } = useQuery({
    queryKey: SHOPPING_LIST_KEY,
    queryFn: fetchShoppingList,
  });

  function invalidate() {
    queryClient.invalidateQueries({ queryKey: SHOPPING_LIST_KEY });
  }

  const generateMutation = useMutation({
    mutationFn: () => generateShoppingList(start, end),
    onSuccess: invalidate,
  });

  const toggleMutation = useMutation({
    mutationFn: ({ id, checked }: { id: string; checked: boolean }) => toggleShoppingItem(id, checked),
    onSuccess: invalidate,
  });

  const clearMutation = useMutation({
    mutationFn: clearCheckedShoppingItems,
    onSuccess: invalidate,
  });

  const clearAllMutation = useMutation({
    mutationFn: clearAllShoppingItems,
    onSuccess: invalidate,
  });

  function handleClearAll() {
    if (window.confirm("Vider toute la liste de courses ?")) {
      clearAllMutation.mutate();
    }
  }

  const addMutation = useMutation({
    mutationFn: (text: string) => addShoppingItem(text),
    onSuccess: () => {
      invalidate();
      setNewItemText("");
      setAddingItem(false);
    },
  });

  function submitNewItem() {
    if (newItemText.trim()) {
      addMutation.mutate(newItemText.trim());
    } else {
      setAddingItem(false);
    }
  }

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Liste de courses</h1>
          <p className={styles.subtitle}>Générée depuis le planning du {weekLabel}.</p>
        </div>
        <div className={styles.headerActions}>
          <button className={styles.clearButton} onClick={() => clearMutation.mutate()}>
            Effacer les articles cochés
          </button>
          <button className={styles.clearAllButton} onClick={handleClearAll}>
            Vider la liste
          </button>
          {addingItem ? (
            <input
              autoFocus
              className={styles.addInput}
              value={newItemText}
              placeholder="Nom de l'article…"
              onChange={(e) => setNewItemText(e.target.value)}
              onBlur={submitNewItem}
              onKeyDown={(e) => e.key === "Enter" && submitNewItem()}
            />
          ) : (
            <button className={styles.addButton} onClick={() => setAddingItem(true)}>
              + Ajouter un article
            </button>
          )}
        </div>
      </div>

      {isLoading && <p className={styles.status}>Chargement…</p>}

      {!isLoading && list && list.categories.length === 0 && (
        <div className={styles.emptyState}>
          <p>La liste est vide.</p>
          <button className={styles.generateInline} onClick={() => generateMutation.mutate()}>
            Générer depuis le planning du {weekLabel}
          </button>
        </div>
      )}

      {!isLoading && list && list.categories.length > 0 && (
        <div className={styles.body}>
          <div className={styles.categories}>
            {list.categories.map((cat) => (
              <div key={cat.name} className={styles.category}>
                <span className={styles.categoryName}>{cat.name}</span>
                {cat.items.map((item) => (
                  <div key={item.id} className={styles.itemRow}>
                    <input
                      type="checkbox"
                      checked={item.checked}
                      onChange={(e) => toggleMutation.mutate({ id: item.id, checked: e.target.checked })}
                      className={styles.checkbox}
                    />
                    {item.quantity && <span className={styles.itemQty}>{item.quantity}</span>}
                    <span className={item.checked ? styles.itemNameChecked : styles.itemName}>{item.food}</span>
                  </div>
                ))}
              </div>
            ))}
          </div>

          <div className={styles.sidebar}>
            <div className={styles.summaryCard}>
              <span className={styles.summaryLabel}>Résumé</span>
              <span className={styles.summaryValue}>{list.total_items} articles</span>
              <span className={styles.summaryDetail}>
                {list.categories.length} catégories · {list.recipe_count} recette{list.recipe_count !== 1 && "s"} du
                planning
              </span>
            </div>
            <div className={styles.infoCard}>
              <span className={styles.infoTitle}>Fusion automatique</span>
              <span className={styles.infoText}>
                Les quantités identiques sont additionnées automatiquement entre les recettes du planning.
              </span>
            </div>
            <button className={styles.generateButton} onClick={() => generateMutation.mutate()}>
              Régénérer depuis le planning
            </button>
            <button className={styles.shareButton}>Partager la liste</button>
          </div>
        </div>
      )}
    </>
  );
}
