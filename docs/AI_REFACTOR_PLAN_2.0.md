# Plan de durcissement post-2.0.3

Ce plan est conçu pour être exécuté tâche par tâche par une IA peu autonome.
Ne pas mélanger plusieurs tâches dans un même commit. Ne pas modifier le
comportement métier hors du périmètre décrit.

## Règles de validation communes

Après chaque tâche, exécuter :

```powershell
npm run build
npm run lint
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo test --manifest-path src-tauri\Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
```

## 1. Fiabiliser le rollback de restauration

Problème : `src-tauri/src/backups/mod.rs::restore_backup` restaure l'ancienne
base si le renommage ou la reconnexion échoue, mais pas si
`recover_admin_access` échoue. L'appelant reçoit alors une erreur alors que la
nouvelle base reste active et que l'ancienne reste dans le fichier rollback.

Étapes :

1. Extraire une fonction privée qui restaure le fichier rollback et reconnecte
   l'ancienne base.
2. Appeler cette fonction pour toute erreur survenant après le remplacement du
   fichier, y compris l'échec de `recover_admin_access`.
3. Ne supprimer le fichier rollback qu'après validation complète.
4. Ajouter des tests Rust avec un répertoire temporaire couvrant :
   remplacement réussi, reconnexion impossible, récupération admin impossible.

Critère d'acceptation : après chaque échec simulé, le chemin principal contient
exactement la base d'origine et aucun fichier rollback orphelin ne subsiste.

## 2. Renouveler les verrous d'édition

Problème : `src/hooks/useEditLock.ts` acquiert un verrou de 10 minutes une seule
fois. Une fiche laissée ouverte plus longtemps peut être modifiée par un autre
poste.

Étapes :

1. Ajouter une commande backend explicite de renouvellement, ou réutiliser
   `acquire_edit_lock` sans changer son contrat.
2. Dans `useEditLock`, renouveler le verrou toutes les 4 minutes tant que le
   composant est monté et que le verrou appartient à cette instance.
3. Si le renouvellement échoue ou retourne `acquired: false`, passer
   immédiatement la fiche en lecture seule et afficher une erreur.
4. Garantir que le timer est annulé et le verrou libéré au démontage.
5. Ajouter des tests Rust sur expiration, renouvellement et refus d'un token
   différent.

Critère d'acceptation : une fiche ouverte 15 minutes reste verrouillée par son
éditeur; après fermeture, un autre poste peut l'acquérir immédiatement.

## 3. Ajouter des tests frontend ciblés

Problème : les hooks et composants génériques récemment introduits ne sont
couverts par aucun test frontend.

Étapes :

1. Installer Vitest, jsdom et React Testing Library.
2. Ajouter un script `test` dans `package.json` et l'exécuter dans les workflows
   GitHub avant le build.
3. Tester `useAsyncData` : succès, erreur, requête obsolète, `immediate: false`.
4. Tester `TableExportModal` : attente de la promesse, erreur affichée, fermeture
   uniquement après succès.
5. Tester l'ouverture d'un contact depuis `CategoriesPage` en vérifiant que
   `get_personne` est appelé avant l'ouverture de la modale.

Critère d'acceptation : les régressions ci-dessus font échouer les tests avant
correction et tous les tests passent après correction.

## 4. Centraliser les référentiels optionnels

Problème : plusieurs pages chargent catégories, personnes, structures et
fonctions avec des variantes locales. Les rôles restreints peuvent recevoir des
listes vides sans explication.

Étapes :

1. Créer un hook dédié qui charge chaque référentiel indépendamment.
2. Ne lancer un chargement que si la session possède la permission `*.read`
   correspondante.
3. Retourner séparément données, chargement et erreur pour chaque référentiel.
4. Migrer `SearchPage`, `StructuresPage`, `ContactModal` et `ReunionModal`.
5. Supprimer uniquement la duplication remplacée par ce hook.

Critère d'acceptation : l'échec ou l'absence de permission pour un référentiel
n'efface jamais les référentiels autorisés.

## 5. Réduire le bundle frontend

Problème : le build signale un chunk JavaScript supérieur à 500 kB et un import
dynamique Tauri inefficace.

Étapes :

1. Mesurer les modules principaux avec l'analyseur de bundle Vite.
2. Charger paresseusement les pages lourdes depuis `App.tsx`.
3. Corriger ou supprimer l'import dynamique inefficace dans `src/lib/tauri.ts`.
4. Ne pas relever artificiellement la limite d'avertissement.

Critère d'acceptation : aucun chunk applicatif principal ne dépasse 500 kB
minifié et le warning d'import dynamique disparaît.
