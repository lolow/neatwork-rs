| EPANET quantity | INP SI | INP US | NeatWork canonical |
|---|---:|---:|---:|
| Elevation | m | ft | m |
| Pipe length | m | ft | m |
| Pipe diameter | mm | in | m |
| Demand/Flow | selected flow unit | selected flow unit | L/s |
| Hydraulic head | m | ft | m |
| Pressure | m | psi | m |
| Velocity | m/s | ft/s | m/s |
| Emitter coefficient | flow/√m | flow/√psi | Adapter required |

| EPANET | NeatWork | Mapping |
|---|---|---|
| Junction ID | `nodes.noeud` | ✅ Direct |
| Junction Elevation | `nodes.haut` | ✅ Direct + units |
| Coordinates | `xdata/ydata` | ✅ Direct |
| Reservoir | Source node | 🟡 Semantic |
| Pipe Node1/Node2 | `pred/succ` | ✅ Direct |
| Pipe Length | `lon/lon1` | ✅ Adapter |
| Pipe Diameter | catalog + `ID1` | 🟡 Adapter |
| Roughness | catalog/pipe roughness | 🟡 Depends on HEADLOSS |
| Base Demand | `clients.cible` | 🟡 Semantic |
| Opening probability | `clients.proba` | ❌ Missing in EPANET |
| Multiple demands | multiple clients/profile | 🟡 Research |
| Emitter | NeatWork orifice | 🟡 Not equivalent |
| Pattern | — | 🟡 Preserve |
| Tank | — | ❌ Unsupported |
| Pump | — | ❌ Unsupported |
| Valve | — | ❌ Unsupported |
| Controls/Rules | — | ❌ Unsupported |
| Vertices | graph geometry | 🟡 Future |