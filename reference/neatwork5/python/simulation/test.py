from data import *
from utils import *
from solver import *
from simulation import *
import time

if __name__ == "__main__":
    nom_instance = "Toy"
    version = "blu"
    sortie = "dsg"
    fichier = "Toy_blu_Secu1.00_fusion0_split0"
    dossier = "examples/dsg_examples"
    
    print(f"Chargement du réseau {fichier}...")
    reseau = data(nom_instance, version, sortie, dossier, fichier)
    reseau.completeTables()
    reseau.complete_data()
    
    reseau.clients['proba'] = 1
    print(f"Préparation de {reseau.param['nbsim']} scénarios probabilistes...")
    sim = simulation(reseau)
    sim.runSimul()
    sim.compute_stats()
    sim.compare_stats()


    version = "bbu"
    sortie = "dsg"
    fichier = "Toy_bbu_Secu1.00_fusion0_split0"

    print(f"Chargement du réseau {fichier}...")
    reseau = data(nom_instance, version, sortie, dossier, fichier)
    reseau.completeTables()
    reseau.complete_data()
    
    reseau.clients['proba'] = 1
    print(f"Préparation de {reseau.param['nbsim']} scénarios probabilistes...")
    sim = simulation(reseau)
    sim.runSimul()
    sim.compute_stats()
    sim.compare_stats()


    version = "bbd"
    sortie = "dsg"
    fichier = "Toy_bbd_Secu1.00_fusion0_split0"
    
    print(f"Chargement du réseau {fichier}...")
    reseau = data(nom_instance, version, sortie, dossier, fichier)
    reseau.completeTables()
    reseau.complete_data()
    reseau.clients['proba'] = 1
    print(f"Préparation de {reseau.param['nbsim']} scénarios probabilistes...")
    sim = simulation(reseau)
    sim.runSimul()
    sim.compute_stats()
    sim.compare_stats()
    
