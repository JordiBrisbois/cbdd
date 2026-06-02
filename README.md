# CRVI-GRC

**CRVI - Gestion de la Relation Contact**

Application de bureau CRM développée pour la gestion centralisée de contacts, structures, affiliations, réunions et conformité RGPD.

---

## Table des matières

1. [Vue d'ensemble](#vue-densemble)
2. [Architecture technique](#architecture-technique)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Guide d'utilisation](#guide-dutilisation)
6. [Système de permissions](#système-de-permissions)
7. [Base de données](#base-de-données)
8. [Développement](#développement)
9. [Build et distribution](#build-et-distribution)
10. [Changelog](#changelog)

---

## Vue d'ensemble

### Fonctionnalités principales

| Module | Description |
|--------|-------------|
| **Contacts (Personnes)** | Gestion des contacts avec civilité, coordonnées, adresse, consentement RGPD |
| **Structures** | Gestion des organisations avec type, catégorie, adresse, contacts |
| **Affiliations** | Liaison personne ↔ structure avec fonction, catégorie, titre spécifique, email pro |
| **Catégories** | Classification des structures et affiliations |
| **Fonctions** | Rôles professionnels dans les structures |
| **Réunions** | Planification avec présences et statuts |
| **RGPD** | Suivi du consentement et anonymisation |
| **Recherche avancée** | Query Builder avec filtres multi-critères et export |
| **Colonnes personnalisables** | Réorganisation par drag & drop, colonnes sticky, visibilité |
| **Export Excel historique** | Reconstruction d'un classeur `.xlsx` par catégorie depuis SQLite |
| **Sauvegardes SQLite** | Backups locaux automatiques/manuels avec restauration assistée |
| **Sécurité multi-postes** | Verrous d'édition backend + détection de conflit sur sauvegarde |

### Stack technique

| Couche | Technologie |
|--------|-------------|
| **Frontend** | React 19 + TypeScript + Tailwind CSS v4 + Vite |
| **Backend** | Rust + Tauri v2 |
| **Base de données** | SQLite (fichier unique) |
| **Auth** | Sessions locales avec Argon2 |
| **UI** | Composants custom (pas de lib externe) |
| **Drag & Drop** | `@hello-pangea/dnd` |

---

## Architecture technique

### Structure du projet

```
crvi-bdd/
├── src/                          # Frontend React
│   ├── components/               # Composants réutilisables
│   │   └── DataTable.tsx         # Tableau avancé (sort, pagination, DnD, sticky)
│   ├── pages/                    # Pages principales
│   │   ├── ContactsPage.tsx
│   │   ├── StructuresPage.tsx
│   │   ├── CategoriesPage.tsx
│   │   ├── ReunionsPage.tsx
│   │   ├── RGPDPage.tsx
│   │   ├── SearchPage.tsx        # Query Builder
│   │   └── AdminPage.tsx         # Gestion utilisateurs/rôles
│   ├── modals/                   # Modals d'édition
│   │   ├── ContactModal.tsx
│   │   ├── StructureModal.tsx
│   │   ├── AffiliationModal.tsx
│   │   ├── ReunionModal.tsx
│   │   └── CategoriesModal.tsx
│   ├── lib/                      # Utilitaires
│   │   ├── auth.tsx              # Système d'authentification
│   │   ├── columns.ts            # Hooks colonnes (visibilité, ordre, sticky)
│   │   ├── utils.tsx             # Fonctions utilitaires + UI
│   │   └── ui.tsx                # Composants UI de base
│   └── types.ts                  # Types TypeScript
├── src-tauri/
│   ├── src/
│   │   ├── main.rs               # Point d'entrée Tauri
│   │   ├── lib.rs                # Enregistrement des commandes
│   │   ├── commands.rs           # Commandes Tauri (CRUD + business logic)
│   │   ├── models.rs             # Structs Rust ↔ DB
│   │   ├── auth.rs               # Authentification & permissions
│   │   ├── backups.rs            # Backups/restaurations SQLite locales
│   │   ├── excel_export.rs       # Reconstruction Excel historique
│   │   └── db.rs                 # Connexion SQLite
│   └── Cargo.toml
├── .github/workflows/            # CI/CD GitHub Actions
│   ├── build.yml
│   └── release.yml
└── CRVI_GRC_be.sqlite            # Base de données (non versionnée)
```

### Communication Frontend ↔ Backend

Le frontend communique avec le backend Rust via les **commandes Tauri** : `invoke(cmd, args)`

```typescript
// Exemple : lister les personnes
const personnes = await invoke<Personne[]>("lister_personnes", { recherche: "Dupont" });

// Exemple : sauvegarder
await invoke("sauvegarder_personne", { personne: form });
```

Chaque commande Rust vérifie automatiquement les permissions via `auth::require_permission(&conn, "code.permission")`.

---

## Installation

### Prérequis

- **Node.js** >= 22
- **Rust** (via [rustup](https://rustup.rs/))
- **Tauri dependencies** (voir [Tauri prerequisites](https://tauri.app/start/prerequisites/))
  - Windows : `WebView2`, `Visual Studio Build Tools`
  - Linux : `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, etc.

### Cloner et installer

```bash
git clone https://github.com/VoxSake/crvi-bdd.git
cd crvi-bdd
npm install
```

### Lancer en mode développement

```bash
npm run tauri dev
```

Lance Vite sur `http://localhost:5173` + le backend Rust en parallèle.

### Lancer uniquement le frontend (web)

```bash
npm run dev
```

> **Note** : Sans le backend Rust, les appels `invoke()` échoueront. Utile uniquement pour tester l'UI.

---

## Configuration

### Données sensibles et fichiers locaux

Le dépôt ne doit pas contenir de données métier réelles, d'exports utilisateurs, de bases SQLite de production, de sauvegardes locales ni de documents internes.

À garder hors Git :

- bases `*.sqlite`, `*.db`, `*.sqlite3`
- dossiers `portable/` générés localement
- rendus et documents de travail sous `docs/`
- sauvegardes locales Tauri et scripts de recovery générés sur poste

### Base de données

L'application utilise un **fichier SQLite unique** (`CRVI_GRC_be.sqlite`).

**Au premier démarrage** : cliquer sur "Choisir une base" et sélectionner le fichier `.sqlite`. Le chemin est sauvegardé automatiquement.

**Auto-découverte** : Si le fichier se trouve dans le même dossier que l'exécutable, il est détecté automatiquement.

### Exploitation multi-postes SQLite

L'application peut fonctionner avec une base SQLite partagée, mais avec des **garde-fous applicatifs** :

- **Mode d'écriture prudent** : `busy_timeout`, `journal_mode=DELETE`, transactions courtes
- **Verrous d'édition** : une fiche ouverte en édition (`personne`, `structure`, `affiliation`) est verrouillée temporairement
- **Détection de conflit** : si un autre poste a modifié la fiche avant l'enregistrement, la sauvegarde est refusée avec message explicite
- **Sauvegardes locales** : backups automatiques/manuels sur les postes autorisés, avec rotation et restauration admin

> **Important** : SQLite sur un partage réseau reste un compromis. Ces protections réduisent les risques, mais ne remplacent pas un vrai serveur de base de données.

### Authentification

- **Mode par défaut** : l'utilisateur `admin` (mot de passe à définir au premier lancement) a tous les droits.
- **Rôles personnalisables** : via la page Admin, créer des rôles avec des permissions granulaires.
- **Connexion anonyme** : peut être activée dans les paramètres de sécurité (Admin → Réglages sécurité).

### Paramètres persistés

| Type | Clé localStorage | Description |
|------|------------------|-------------|
| Visibilité colonnes | `crvi-columns-{tableId}` | Colonnes visibles par tableau |
| Colonne sticky | `crvi-sticky-{tableId}` | Colonne figée |
| Ordre colonnes | `crvi-order-{tableId}` | Ordre personnalisé |
| Largeur colonnes | `crvi-widths-{tableId}` | Largeurs redimensionnées par tableau |

---

## Guide d'utilisation

### Navigation

| Page | Raccourci / Accès | Description |
|------|-------------------|-------------|
| **Contacts** | Menu principal | Répertoire des personnes avec filtres |
| **Structures** | Menu principal | Répertoire des organisations |
| **Catégories** | Menu principal | Vue groupée par catégorie (personnes + structures) |
| **Réunions** | Menu principal | Planification et gestion des présences |
| **RGPD** | Menu principal | Suivi du consentement et anonymisation |
| **Recherche** | Menu principal | Query Builder avancé |
| **Admin** | Menu principal (admin uniquement) | Utilisateurs, rôles, paramètres |

### Administration

La page **Admin** centralise maintenant :

- la gestion des comptes et rôles
- les réglages de sécurité
- la reconstruction d'un **Excel historique** depuis SQLite
- les **sauvegardes SQLite locales** : création manuelle, liste locale, suppression, restauration assistée

### Sauvegardes SQLite

- Les backups automatiques ne s'exécutent que sur les postes ayant la permission `admin.backups`
- Un garde-fou global évite que plusieurs postes autorisés créent tous le même backup horaire
- Les backups sont stockés **localement** dans le dossier applicatif Tauri du poste
- Rotation automatique : **10 backups max** par base locale
- La page Admin permet aussi de **supprimer un backup local** si besoin, avec la même permission `admin.backups`
- Une restauration crée d'abord une copie `pre_restore_*` avant remplacement de la base active

### Export Excel historique

- Bouton admin : **Reconstruire le fichier Excel**
- Génère un `.xlsx` avec **une sheet par catégorie métier**
- S'appuie d'abord sur la **catégorie de l'affiliation** pour rester cohérent avec l'ancien fichier Excel
- Ajoute aussi les **structures sans référent** quand c'est pertinent

### Tableaux interactifs

Tous les tableaux partagent les mêmes fonctionnalités :

- **Tri** : clic sur l'en-tête de colonne (asc/desc)
- **Pagination** : 50/100/250/500/1000 lignes par page
- **Colonnes visibles** : bouton "Colonnes" → cocher/décocher
- **Redimensionner** : tirer la poignée à droite d'un en-tête de colonne
- **Réorganiser** : drag & drop sur les en-têtes (sauf colonne figée)
- **Figer** : clic sur la punaise dans le menu colonnes → bloc figé à gauche
- **Réinitialiser** : boutons de remise à zéro de l'ordre et des largeurs dans le menu
- **Exporter CSV** : bouton "Exporter" (exporte les colonnes visibles)

### Modals d'édition

**Contact** :
- Formulaire d'identité + coordonnées
- Section **Affiliations** : tableau des structures liées avec édition/suppression
- Bouton "Ajouter" → modal Affiliation (mode personne)

**Structure** :
- Formulaire d'identité + coordonnées
- Sélecteur de **catégorie**
- Section **Affiliations** : tableau des personnes référentes
- Bouton "Ajouter" → modal Affiliation (mode structure)
- **Quick-add personne** : créer une nouvelle personne + affiliation en un clic

**Affiliation** :
- Mode **personne** : sélection de la structure + fonction + catégorie
- Mode **structure** : sélection de la personne + fonction + catégorie
- Quick-add : formulaire inline (civilité, nom, prénom, email, téléphone)

---

## Système de permissions

### Permissions disponibles

| Code | Description | Contexte |
|------|-------------|----------|
| `personnes.read` | Consulter les personnes | Pages Contacts, Catégories |
| `personnes.create` | Créer des personnes | Bouton "Nouveau" dans Contacts |
| `personnes.update` | Modifier des personnes | Modal Contact (mode édition) |
| `personnes.delete` | Supprimer des personnes | Bouton "Supprimer" dans modal |
| `structures.read` | Consulter les structures | Pages Structures, Catégories |
| `structures.create` | Créer des structures | Bouton "Nouveau" dans Structures |
| `structures.update` | Modifier des structures | Modal Structure (mode édition) |
| `structures.delete` | Supprimer des structures | Bouton "Supprimer" dans modal |
| `affiliations.read` | Consulter les affiliations | Sections affiliations dans modals |
| `affiliations.create` | Créer des affiliations | Bouton "Ajouter" affiliations |
| `affiliations.update` | Modifier des affiliations | Icône édition dans tableau affiliations |
| `affiliations.delete` | Supprimer des affiliations | Icône suppression dans tableau affiliations |
| `categories.read` | Consulter les catégories | Page Catégories |
| `categories.create` | Créer des catégories | Admin / gestion |
| `categories.update` | Modifier des catégories | Admin / gestion |
| `categories.delete` | Supprimer des catégories | Admin / gestion |
| `reunions.read` | Consulter les réunions | Page Réunions |
| `reunions.create` | Créer des réunions | Page Réunions |
| `reunions.update` | Modifier des réunions | Page Réunions |
| `reunions.delete` | Supprimer des réunions | Page Réunions |
| `presences.read` | Consulter les présences | Page Réunions (détail) |
| `presences.create` | Créer des présences | Page Réunions (détail) |
| `presences.update` | Modifier des présences | Page Réunions (détail) |
| `presences.delete` | Supprimer des présences | Page Réunions (détail) |
| `rgpd.read` | Consulter le RGPD | Page RGPD |
| `rgpd.update` | Modifier les statuts RGPD | Page RGPD |
| `rgpd.anonymize` | Anonymiser une personne | Page RGPD |
| `rgpd.anonymize.bulk` | Anonymiser en masse | Page RGPD |
| `search.read` | Utiliser la recherche avancée | Page Recherche |
| `presets.read` | Consulter les presets | Page Recherche |
| `presets.create` | Créer des presets | Page Recherche |
| `presets.update` | Modifier des presets | Page Recherche |
| `presets.delete` | Supprimer des presets | Page Recherche |
| `admin.users` | Gérer les comptes | Page Admin |
| `admin.roles` | Gérer les rôles | Page Admin |
| `admin.settings` | Gérer les réglages de sécurité | Page Admin |
| `admin.exports` | Exporter les données métier | Page Admin |
| `admin.backups` | Gérer les sauvegardes et restaurations | Page Admin |

### Rôles par défaut

| Rôle | Description | Permissions |
|------|-------------|-------------|
| **admin** | Administrateur | Toutes |
| **public** | Utilisateur anonyme | Définies dans les réglages sécurité |

### Vérification dans le code

**Rust** : chaque commande vérifie la permission
```rust
auth::require_permission(&conn, "structures.read")?;
```

**TypeScript** : le hook `useAuth()` expose `can()`
```typescript
const { can } = useAuth();
const canCreate = can("structures.create");
```

---

## Base de données

### Schéma SQLite

```sql
T_Personnes          -- Contacts (ID_Personne, Nom, Prenom, Email, Tel, Adresse, RGPD...)
T_Structures         -- Organisations (ID_Structure, Nom_Structure, Adresse, Contact...)
T_Affiliations       -- Liens (ID_Affiliation, Ref_Personne, Ref_Structure, Ref_Fonction, ID_Categorie...)
T_Categories         -- Classification (ID_Categorie, Nom_Categorie)
T_Fonctions          -- Rôles (ID_Fonction, Libelle_Fonction)
T_Reunions           -- Réunions (ID_Reunion, Titre, Date, Heure, Lieu, Ref_Structure)
T_Presences          -- Présences (ID_Presence, Ref_Reunion, Ref_Personne, Statut)
T_Permissions        -- Permissions (ID_Permission, Code_Permission, Description)
T_Roles              -- Rôles (ID_Role, Code_Role, Nom_Role)
T_RolePermissions    -- Liens rôle-permission
T_Users              -- Utilisateurs (ID_User, Username, Password_Hash)
T_UserRoles          -- Liens user-rôle
T_AppSettings        -- Paramètres (Setting_Key, Setting_Value)
T_EditLocks          -- Verrous d'édition temporaires multi-postes
T_BackupState        -- Coordination des backups automatiques
T_Presets            -- Presets recherche (ID_Preset, Nom, Table, Colonnes, Conditions)
```

### Protections de concurrence

- `Updated_At` sur `T_Personnes`, `T_Structures`, `T_Affiliations`
- Contrôle optimiste à l'enregistrement : refus si la fiche a changé depuis son ouverture
- Verrou applicatif temporaire pour éviter deux éditions simultanées sur la même fiche

### Indexes créés

| Index | Colonne | Utilité |
|-------|---------|---------|
| `idx_affiliations_personne` | `Ref_Personne` | Jointure personne→affiliations |
| `idx_affiliations_structure` | `Ref_Structure` | Jointure structure→affiliations |
| `idx_affiliations_categorie` | `ID_Categorie` | Filtrage par catégorie |
| `idx_personnes_nom` | `Nom` | Recherche |
| `idx_personnes_prenom` | `Prenom` | Recherche |
| `idx_structures_nom` | `Nom_Structure` | Recherche |
| `idx_presences_reunion` | `Ref_Reunion` | Jointure réunion→présences |
| `idx_presences_personne` | `Ref_Personne` | Jointure personne→présences |

---

## Développement

### Scripts npm

| Commande | Description |
|----------|-------------|
| `npm run dev` | Serveur Vite (frontend uniquement) |
| `npm run build` | Build TypeScript + Vite (production) |
| `npm run tauri dev` | Développement Tauri complet |
| `npm run tauri build` | Build binaire Tauri |
| `npm run lint` | ESLint |

### Architecture du code

**Frontend** (`src/`)

| Dossier | Rôle |
|---------|------|
| `components/` | Composants réutilisables (DataTable, CopyValuesModal) |
| `pages/` | Pages écran plein (7 pages principales) |
| `modals/` | Fenêtres modales (5 modals) |
| `lib/` | Logique métier partagée |
| `lib/auth.ts` | Hook `useAuth()`, session, permissions |
| `lib/columns.ts` | Hooks `useColumnVisibility`, `useStickyColumn`, `useColumnOrder` |
| `lib/utils.tsx` | Fonctions utilitaires + composants UI (SortHeader, ColumnToggle, etc.) |

**Backend** (`src-tauri/src/`)

| Fichier | Rôle |
|---------|------|
| `commands.rs` | Commandes Tauri CRUD + admin + verrous + backups |
| `models.rs` | Structs Rust ↔ sérialisation JSON ↔ DB |
| `auth.rs` | Authentification, sessions, rôles, permissions |
| `backups.rs` | Création, rotation, vérification et restauration des backups SQLite |
| `excel_export.rs` | Reconstruction d'un classeur Excel historique depuis SQLite |
| `db.rs` | Connexion SQLite, path persistence |
| `lib.rs` | Point d'entrée, enregistrement des commandes |

### Ajouter une nouvelle commande

1. **Rust** (`commands.rs`) : implémenter la fonction avec `#[tauri::command]`
2. **Rust** (`lib.rs`) : l'ajouter dans `generate_handler![]`
3. **TypeScript** (`types.ts`) : ajouter les types si nécessaire
4. **Frontend** : appeler via `invoke("ma_commande", { args })`

---

## Build et distribution

### Build local

```bash
npm run tauri build
```

Génère :
- `src-tauri/target/release/bundle/msi/*.msi` — Installateur Windows MSI
- `src-tauri/target/release/bundle/nsis/*.exe` — Installateur Windows NSIS
- `src-tauri/target/release/crvi-grc.exe` — Binaire portable

### CI/CD GitHub Actions

**Build** (`.github/workflows/build.yml`) : à chaque push sur `main`
- Build Windows avec Rust + Node 22
- Upload du binaire en artifact

**Release** (`.github/workflows/release.yml`) : à chaque tag `v*`
- Build + signature des bundles
- Création de `latest.json` (mise à jour auto)
- Publication GitHub Release avec MSI + NSIS

### Version bump

Modifier dans 5 fichiers :
1. `package.json` → `"version": "x.y.z"`
2. `package-lock.json` → `"version": "x.y.z"`
3. `src-tauri/Cargo.toml` → `version = "x.y.z"`
4. `src-tauri/Cargo.lock` → entrée du package `crvi-grc`
5. `src-tauri/tauri.conf.json` → `"version": "x.y.z"`

Puis :
```bash
git add . && git commit -m "Bump vX.Y.Z"
git tag vX.Y.Z
git push origin main
git push origin refs/tags/vX.Y.Z
```

La release est créée automatiquement par GitHub Actions.

---

## Changelog

Le changelog détaillé est volontairement maintenu hors README pour éviter d'y accumuler des détails métier et historiques.

Utiliser :

- l'historique Git pour les changements techniques
- les tags `v*` pour les versions publiées
- les releases GitHub pour les livrables signés

---

## Licence

**MIT License** — voir le fichier [`LICENSE`](LICENSE)

### Pourquoi MIT ?

MIT a été choisi car c'est la licence **la plus permissive et la moins contraignante** :

- Utilisation commerciale ou privée : ✅ autorisée
- Modification : ✅ autorisée
- Distribution : ✅ autorisée
- Sous-licence : ✅ autorisée
- Seule obligation : garder le copyright et le texte de la licence

### MIT vs Apache 2.0

| Critère | MIT | Apache 2.0 |
|---|---|---|
| Utilisation | ✅ Libre | ✅ Libre |
| Modification | ✅ Libre | ✅ Libre (changements à documenter) |
| Brevets | Non mentionné | ✅ Protection explicite |
| Obligations | Copyright + licence | Copyright + licence + NOTICE + changements |
| Longueur | ~20 lignes | ~200 lignes |

MIT est plus courte et plus simple. Apache 2.0 offre une protection brevet mais impose plus de contraintes administratives.

---

## Support

Pour toute question ou problème, ouvrir une issue sur le dépôt GitHub.
