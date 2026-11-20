from data import *
from utils import *
from solver import *
from simulation import *
import time
"""
Simuler un design à partir d'un fichier .dsg et stocker le résultat dans un
fichier .xlsx. Le fichier de données de design doit être préparé au préalable.
Donner le nom, la version du réseau et le coefficient de sécurité
Le fichier dsg se trouve dans le dossier NW-Git/out/'nom'/ sous le nomv
qui est le mom du réseau complété par la version et suivi de _Secu avec
la valeur du coefficient de sécurité (>= 1).

1. CREER le nom et l'adresse du fichier de données de design. Ces
éléments contiennent le type du fichier (.dsg, .mat ou.json).
2. RECUPERER le fichier par son adresse. On extrait les tables TA, TN,
TC. Puis TD (base de données des diamètres), TO (base de données des
orifices).  Les paramètres sont récupérés à partir de TD (cas d'un fichier
dsg) et transformé en structure param, soit donnés directement en
structures (cas .mat et .json). Créer une structure dat
3. COMPLETER les éléments manquants.
Remarque : la base de données TO n'est pas utile pour la simulation
"""

if __name__ == "__main__":
    nom_instance = "Esperenza"                        # nom de l'instance à simuler et du sous-dossier où elle se trouve
    version = "2S"                             # version du réseau à simuler (ex: bbu, blu, bbd)
    sortie = "dsg"                              # type de fichier de données d'entrée (dsg, mat ou json)
    fichier = "Esperenza2S"
    dossier = "examples/dsg_examples"           # nom du dossier parent. Par défaut out. Pour débogage 'examples'.
    seed = 42                                   # seed pour la génération des scénarios probabilistes
    
    print(f"Chargement du réseau {fichier}...")
    reseau = data(nom_instance, version, sortie, dossier, fichier)
    print("Voulez-vous revoir les paramètres de simulation ? (y/n)")
    if input().lower() == 'y':
        reseau.review_simul_parameters()
    reseau.completeTables()
    reseau.complete_data()
    print(f"Préparation de {reseau.param['nbsim']} scénarios probabilistes...")
    sim = simulation(reseau, seed)
    
    print("Lancement du solveur...")
    t_start = time.time()
    # Lancement des simulations pour tous les scénarios et stockage des résultats dans sim.sim_res
    sim.runSimul()
    t_end = time.time()
    
    sim.sim_res['TempsIPM'] = t_end - t_start
    
    iters = sim.sim_res["Iter"]
    succes = iters[iters > 0]
    echecs = np.sum(iters == -1)
    
    print("\n--- Bilan de la simulation ---")
    print(f"Scénarios résolus : {len(succes)}/{reseau.param['nbsim']}")
    print(f"Échecs (non-convergence) : {echecs}")
    if len(succes) > 0:
        print(f"Moyenne d'itérations par succès : {np.mean(succes):.1f}")
        
    print("\n--- Traitement Statistique ---")
    print("Souhaitez-vous afficher les statistiques à l'écran ? (y/n)")
    print_flag = input().lower() == 'y'
    print("Souhaitez-vous exporter les statistiques vers un fichier ? (y/n)")
    export_flag = input().lower() == 'y'
    sim.stat_processing(print_flag, export_flag)