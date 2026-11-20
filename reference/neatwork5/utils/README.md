# Utilitaires MATLAB

Ce dossier regroupe quelques fonctions d'aide autour de la préparation de tables et de la validation de structures arborescentes. Elles sont écrites en MATLAB/Octave et peuvent être appelées directement depuis vos scripts.

## `normalize_id_column.m`
Normalise une colonne d'identifiants pour garantir une représentation homogène sous forme de `string` :

- accepte les principaux types MATLAB (tableaux de caractères, `cellstr`, `categorical`, numériques) ;
- applique un `strtrim` pour supprimer les espaces superflus ;
- permet d'imposer une casse (`as-is`, `upper`, `lower`) ;
- gère les valeurs manquantes avec différentes politiques (`error`, `drop`, `fill`).

Cette fonction est utile avant de réaliser des jointures ou des comparaisons d'identifiants.

## `prepTable.m`
Convertit les colonnes d'une table MATLAB afin qu'elles puissent être exportées facilement (par exemple en JSON) :

- les `categorical` sont transformées en `string` ;
- les `datetime` sont formatées en ISO 8601 (`uuuu-MM-dd'T'HH:mm:ss.SSS`) ;
- les `duration` deviennent des secondes ;
- les `string` vides remplacent les valeurs manquantes pour éviter `<missing>`.

## `prepAny.m`
Applique une préparation récursive sur un objet hétérogène (`struct`, `cell`, `table`, `sparse`, etc.) en appelant `prepTable` pour les tables internes. Le résultat est composé uniquement de types simples et sérialisables :

- les tableaux clairsemés (`sparse`) sont convertis en structure contenant les indices et valeurs ;
- les `categorical` et `datetime` sont convertis en chaînes ;
- les `duration` deviennent des secondes ;
- les sous-structures sont parcourues récursivement.

## `subset_struct.m`

Crée une sous-structure de S ne comprenant que les éléments de keep
- Input S structure, keep: cell array des champs à garder. Keep must be a cell array of character vector.
- Output: sous-structure avec les champs de keep.

## `validate_tree.m`
Vérifie qu'une table d'arcs (`TA`, avec les colonnes `pred` et `succ`) et une table de nœuds (`TN`, avec la colonne `noeud`) définissent bien un arbre orienté parent→enfant. La fonction retourne :

- `ok` : booléen vrai si l'arbre est valide ;
- `report` : structure décrivant l'éventuelle erreur détectée (nœuds absents, boucles, cardinalité, racine manquante, indegree invalide, non-connexité).

Vous pouvez fournir explicitement l'identifiant de la racine en troisième argument, sinon la fonction tente de l'inférer via `TN.statut == 0`.

---

### Conseils d'utilisation
- Avant d'appeler `validate_tree`, utilisez `normalize_id_column` pour harmoniser les identifiants de vos tables `TA` et `TN`.
- Pour exporter une structure complexe vers JSON, combinez `prepAny` avec `jsonencode`.

### Compatibilité
Ces fonctions reposent uniquement sur des fonctionnalités standard de MATLAB (R2019b) et devraient également fonctionner sous Octave récent, sous réserve de disposer des fonctions équivalentes (`inputParser`, `table`, etc.).