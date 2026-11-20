# Glossary

Terms of the Java program, their meaning, and their names in the Rust port.

| Java term | Meaning | Rust name |
| --- | --- | --- |
| `Topographie` (`.tpo`) | Topography: the network before design. | `project::Topographie` |
| `Design` (`.dsg`) | Design: the network with diameters and orifices. | `project::Design` |
| `Simulation` (`.sim`) | A design with flow and head statistics. | output of `simulate` |
| `Database` | Diameter and orifice catalogs. | `project::Database` |
| `Node`, `Pipe` (project) | Editable node and pipe of a project file. | `project::network::{Node, Pipe}` |
| `TYPE_RESERVOIR`, `TYPE_DISPATCH`, `TYPE_FAUCET` | Source, branching node, faucet node (0, 1, 2). | `project::network::TYPE_*` |
| expanded topography (`nodeList2`) | Network where each tap of a multi-tap node is a faucet `<node>_a`, `<node>_b`. | `Topographie::expanded_nodes` |
| `Nodes`, `Pipes`, `Taps` (core) | Engine view of nodes, pipes and taps. | `engine::defs::{CNode, CPipe, CTap}` |
| `Diameters` (core) | Engine view of a commercial pipe, with flow coefficients p, q, beta. | `engine::defs::CDiameter` |
| `dvector`, `ovector` | Diameter and orifice lists of the engine. | `Vec<CDiameter>`, `Vec<f64>` |
| `pression` (diameter) | Maximum head that a pipe stands ("max length" column of the catalog). | `CDiameter::pression` |
| `rugosite` | Roughness of a pipe (mm). | `CDiameter::rugosite` |
| `hsource` | Height of the source. The design moves all heights so that the source is at 0. | `hsource` |
| load factor (`loadfactor`) | Factor that gives the design flow of a pipe from the number of taps downstream. | `Topographie::load_factor`, `Design::load_factor` |
| `Decoupe` | Split of a pipe into at most two diameters, rounded to commercial lengths. | `engine::make_design::decoupe` |
| `lcom` (`pipelength`) | Length of one commercial pipe. | `lcom` |
| ideal orifice (`orif_ideal`) | Orifice that gives the target flow exactly. | `CTap::orif_ideal` |
| commercial orifice (`orif_com`) | Nearest orifice of the catalog. 10000 means none. | `CTap::orif_com` |
| `faucetcoef` (alpha) | Loss coefficient of a faucet. | `topo.faucetcoef.value` |
| `orifcoef` | Discharge coefficient of an orifice. | `topo.orifcoef.value` |
| `seuil`, `seuil2` (simulation) | Minimum and maximum critical flows. | `simu.mincriticalflow.value`, `simu.maxcriticalflow.value` |
| `nbsim` | Number of runs of a simulation. | `simu.nbsim.value`, `--nbsim` |
| `simopentaps` | Probability that a tap is open in a random run. | `simu.simopentaps.value`, `--opentaps` |
| `typesimu` | `random`, `tapbytap` or `handmade`. | `SimulationType` |
| failure | A run where a tap flow is below 0.005 l/s. | `CPipe::failure` |
| `S`, `Sb` | Path matrices: intermediate node on the path of a tap, or of another intermediate node. | `path_matrix`, `path_matrix_interm` |
| `Dat` | Data of one flow computation. | `engine::simul_flows::Dat` |
| `SolverDisk` | Local solver entry point of the Java program. | `design`, `simulate` in `lib.rs` |
