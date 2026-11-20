# InOut

Fonctions d’entrée/sortie pour le format `.tpo` et `.dsg`.

## Contenu
- `data4DsgFile` : écrit le fichier `.dsg` correspondant à l'instance créée en faisant appel pour l'écriture csv à export_neatwork_csv
– 'export_neatwork_csv.m' : enregistre dans un fichier les données de sortie préparées.
– 'getSectionBlock.m' : extrait du block de lignes les éléments d'une section.
- `load_dsg_json' : lit les données tables et structure du fichier json
- `parseDsgToTables.m` : transforme les lignes en tables (TN, TA, TC, TP, TD, TO) et structure param.
- `parseTpoToTables.m` : transforme les lignes en tables (TN, TA, TP, TD, TO).
- `apply_client_data.m` : à l'import, recalcule les effectifs depuis TC et en déduit les paramètres de demande uniforme avant leur revue.
- `read_or_build_TC.m` : construit la table des clients lorsqu'elle est absente
- `readDsgFile.m` : lit un fichier `.dsg` brut → vecteur de lignes.
- `readTpoFile.m` : lit un fichier `.tpo` brut → vecteur de lignes.
- `save_dsg_json.n' : sauvegarde tables et structures qui définissent un design
- `validate_and_align_Tables.m' : vérifie absence de noeuds et aligne les catégories dans TA et TN 

## Notes
- Avant la revue des paramètres, TC fait autorité sur les effectifs et la demande uniforme. Après la revue, les paramètres retenus actualisent TC pour la demande uniforme. Seul le TPO construit TC si elle manque ; le DSG exige une section `!Clients`.
- Tous les chemins passent par `ascend_to_repo()` pour trouver la racine NW_Git.
- Les fonctions ne modifient jamais les formats `.tpo`/.`dsg` (compatibilité NW).
- Les variables suivantes sont déclarées du type categorical : TN.noeud, TA.pred, TA.succ, TD.ID, TO.ID après lecture des fichiers de base, tant pour parseTpoToTables (dans makeDesign) que dans parseDsgToTables (dans runSimulation). Il reste des tests de vérification du type dans plusieurs autres fonctions qui ne sont pas nécessaires.
- Si le dossier d'export dans `out/` est manquant, un dossier du nom de l'instance est créé automatiquement.
- Dans l'export le statut des arcs est `'N', 'B' ou 'R'` est conservé.
- L'export en format `MAT` est supprimé. Les deux seuls formats sont `DSG ou JSON`.
