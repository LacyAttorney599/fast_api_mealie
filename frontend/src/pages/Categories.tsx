import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CourseType } from "../lib/api";
import {
  fetchCookbookCategories,
  fetchCourseTypes,
  setCourseType,
} from "../lib/api";
import styles from "./Categories.module.css";

const COURSE_TYPE_LABELS: Record<CourseType, string> = {
  plat: "Plat principal",
  entrée: "Entrée",
  dessert: "Dessert",
  accompagnement: "Accompagnement",
};

const COURSE_TYPE_OPTIONS: CourseType[] = ["plat", "entrée", "dessert", "accompagnement"];

export default function Categories() {
  const queryClient = useQueryClient();

  const { data: categories, isLoading } = useQuery({
    queryKey: ["cookbook-categories"],
    queryFn: fetchCookbookCategories,
  });

  const { data: courseTypes } = useQuery({
    queryKey: ["course-types"],
    queryFn: fetchCourseTypes,
  });

  const mutation = useMutation({
    mutationFn: ({ name, type }: { name: string; type: CourseType | null }) =>
      setCourseType(name, type),
    onSuccess: (data) => {
      queryClient.setQueryData(["course-types"], data);
    },
  });

  function handleChange(categoryName: string, value: string) {
    const type = value === "" ? null : (value as CourseType);
    mutation.mutate({ name: categoryName, type });
  }

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Catégories</h1>
          <p className={styles.subtitle}>
            Définissez le type de service de chaque catégorie pour guider le planificateur IA.
          </p>
        </div>
      </div>

      <p className={styles.hint}>
        Les catégories marquées <strong>Dessert</strong> ou <strong>Accompagnement</strong> ne seront
        pas proposées par l'IA comme plat principal lors de la génération du planning.
      </p>

      {isLoading && <p className={styles.loading}>Chargement…</p>}

      <div className={styles.list}>
        {categories?.map((cat) => {
          const currentType = courseTypes?.[cat.name] ?? "";
          return (
            <div key={cat.id} className={styles.row}>
              <span className={styles.categoryName}>{cat.name}</span>
              <div className={styles.typeSelector}>
                {COURSE_TYPE_OPTIONS.map((type) => (
                  <button
                    key={type}
                    className={
                      currentType === type ? styles.typeBtnActive : styles.typeBtn
                    }
                    onClick={() => handleChange(cat.name, currentType === type ? "" : type)}
                    disabled={mutation.isPending}
                  >
                    {COURSE_TYPE_LABELS[type]}
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}
