# Examples

Ensemble de données de topologies de **réseaux d’exemple** pour des tests sur NeatWork v5 (NW5).

## Contenu versionné (sous Git)

- `xxx.tpo` : données topologiques d’un réseau **non dimensionné**.
- `DiamBasexxx.db` : base de diamètres commerciaux associée.
- `orifices.db` : base d’orifices (équipements terminaux).

Ces fichiers servent de **références d’entrée** pour lancer les pipelines.

## Contenu non versionné (généré par les runs)

- `xxx.dsg` : réseau dimensionné par `CreateDsg`.
- `xxx.inp` : export au format **EPANET** du réseau dimensionné.
- Fichiers annexes (`*_fusion.meta.json`, `*_TableFusion.csv`, logs, etc.).

👉 Tous ces fichiers sont ignorés par Git (voir `.gitignore`).

## Organisation

Chaque sous-dossier `examples/<nom_reseau>/` contient :

- les entrées (`.tpo`, `.db`),
- les sorties générées (`.dsg`, `.inp`, sidecars) avec suffixe de version si besoin.

## Notes

- Les chemins d’accès passent par `ascend_to_repo()` pour retrouver la racine `NW_Git/`.
- Les fichiers `.db` (diamètres, orifices) sont propres à chaque jeu d’exemple.

