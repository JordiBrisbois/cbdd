import { useCallback } from "react";
import type { Categorie, Personne, Structure, Fonction } from "../types";
import { invoke } from "../lib/tauri";
import { useAuth } from "../lib/auth";
import { useAsyncData } from "./useAsyncData";

export interface ReferentialResult<T> {
  data: T;
  loading: boolean;
  error: string | undefined;
  reload: () => Promise<T>;
}

type ReferentialOptions = {
  categories?: boolean;
  personnes?: boolean;
  structures?: boolean;
  fonctions?: boolean;
  personnesImmediate?: boolean;
};

export function useReferentials(options: ReferentialOptions = {}) {
  const { can } = useAuth();
  const categoriesEnabled = options.categories !== false && can("categories.read");
  const personnesEnabled = options.personnes !== false && can("personnes.read");
  const structuresEnabled = options.structures !== false && can("structures.read");
  const fonctionsEnabled = options.fonctions !== false && can("affiliations.read");

  const loadCategories = useCallback(
    () => categoriesEnabled ? invoke<Categorie[]>("lister_categories") : Promise.resolve([]),
    [categoriesEnabled],
  );
  const cats = useAsyncData(loadCategories, [], {
    immediate: categoriesEnabled,
    errorMessage: "Impossible de charger les catégories",
  });

  const loadPersonnes = useCallback(
    () => personnesEnabled ? invoke<Personne[]>("lister_personnes") : Promise.resolve([]),
    [personnesEnabled],
  );
  const pers = useAsyncData(loadPersonnes, [], {
    immediate: personnesEnabled && options.personnesImmediate !== false,
    errorMessage: "Impossible de charger les personnes",
  });

  const loadStructures = useCallback(
    () => structuresEnabled ? invoke<Structure[]>("lister_structures") : Promise.resolve([]),
    [structuresEnabled],
  );
  const structs = useAsyncData(loadStructures, [], {
    immediate: structuresEnabled,
    errorMessage: "Impossible de charger les structures",
  });

  const loadFonctions = useCallback(
    () => fonctionsEnabled ? invoke<Fonction[]>("lister_fonctions") : Promise.resolve([]),
    [fonctionsEnabled],
  );
  const foncs = useAsyncData(loadFonctions, [], {
    immediate: fonctionsEnabled,
    errorMessage: "Impossible de charger les fonctions",
  });

  return {
    categories: { data: categoriesEnabled ? cats.data : [], loading: categoriesEnabled && cats.loading, error: categoriesEnabled ? cats.error : undefined, reload: cats.reload } satisfies ReferentialResult<Categorie[]>,
    personnes: { data: personnesEnabled ? pers.data : [], loading: personnesEnabled && pers.loading, error: personnesEnabled ? pers.error : undefined, reload: pers.reload } satisfies ReferentialResult<Personne[]>,
    structures: { data: structuresEnabled ? structs.data : [], loading: structuresEnabled && structs.loading, error: structuresEnabled ? structs.error : undefined, reload: structs.reload } satisfies ReferentialResult<Structure[]>,
    fonctions: { data: fonctionsEnabled ? foncs.data : [], loading: fonctionsEnabled && foncs.loading, error: fonctionsEnabled ? foncs.error : undefined, reload: foncs.reload } satisfies ReferentialResult<Fonction[]>,
    reloadAll: async () => {
      await Promise.allSettled([cats.reload(), pers.reload(), structs.reload(), foncs.reload()]);
    },
  };
}
