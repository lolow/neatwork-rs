from matplotlib import lines
import pandas as pd
import numpy as np
import os
from pathlib import Path
from utils import *
from scipy import sparse
import dtale
import sys

class SimulationData:
	"""
	class to load and process the network data from a .dsg file, and prepare it for the solver.
	Attributes
	----------
	name : str
		network's name
	version : str
		version of the network
	output : str
		type of file for the output
	file_name : str
		name of the file with the data
	nodes : pd.DataFrame
		source, transit and terminal nodes
	pipes : pd.DataFrame
		pipes data
	clients : pd.DataFrame
		client data
	bornes : pd.DataFrame
		bornes data
	diameters : pd.DataFrame
		diameters data
	param : dict
		parameters of the network
	dat : dict
		processed data for the solver
	options : dict
		solver options
	"""
	def __init__(self, name, version, output, dossier, file_name):
		self.name = name
		self.version = version
		self.output = output
		self.dossier = dossier
		self.file_name = file_name
		self.options = self.init_options()
		self.nodes, self.pipes, self.clients, self.bornes, self.diameters, self.param = self.parse_dsg_to_df()

	def __str__(self):
		return (f"Nom de l'instance: {self.name}\n"
				f"Version: {self.version}\n"
				f"Sortie: {self.output}")

	def get_path(self, sub_dir):
		ROOT_DIR = Path(__file__).resolve().parent.parent.parent
		SUB_DIR = ROOT_DIR / sub_dir / self.name
		return SUB_DIR
	
	def parse_dsg_to_df(self):
		dsg_path = self.get_path(self.dossier)
		dsg_file = os.path.join(dsg_path, f"{self.file_name}.dsg")
		if not os.path.exists(dsg_file):
			raise FileNotFoundError(f"Fichier introuvable : {dsg_file}")
		with open(dsg_file, "r") as file:
			raw_lines = [line.strip() for line in file.readlines()]

			idx_client    = get_section_indices(raw_lines, "Clients")
			idx_nodes     = get_section_indices(raw_lines, "Nodes")
			idx_pipes     = get_section_indices(raw_lines, "Pipes")
			idx_prop      = get_section_indices(raw_lines, "Default properties")
			idx_diameters = get_section_indices(raw_lines, "Diameters")
			idx_bornes    = get_section_indices(raw_lines, "Borne")
			
			missing = []
			if idx_client is None: missing.append("Clients")
			if idx_nodes is None: missing.append("Nodes")
			if idx_pipes is None: missing.append("Pipes")
			if idx_prop is None: missing.append("Default properties")
			if idx_diameters is None: missing.append("Diameters")

			if missing:
				raise ValueError(f"Section(s) manquante(s) : {', '.join(missing)}")
			lines = [[val.strip() for val in line.split(',')] for line in raw_lines]

			# ================== NOEUDS (TN) ==================
			# Récupération des sommets du graphe et de leurs propriétés
			cols = ['noeud','haut','xdata','ydata','orif_ideal','orif_com', 'statut']
			df_nodes = pd.DataFrame(lines[idx_nodes[0]:idx_nodes[1]], columns=cols)
			df_nodes[['haut','xdata','ydata','orif_ideal','orif_com', 'statut']] \
				= df_nodes[['haut','xdata','ydata','orif_ideal','orif_com', 'statut']].apply(pd.to_numeric)
			df_nodes['noeud'] = df_nodes['noeud'].astype(str)
			df_nodes = df_nodes.sort_values(by=['statut', 'noeud']).reset_index(drop=True)

			# ================== ARCS (TA) ==================
			# Récupération des arcs du graphe et de leurs propriétés
			# Les arcs sont orientés de pred vers succ
			cols = ['pred','succ','lon', 'lon1', 'ID1', 'lon2', 'ID2', 'N']
			df_pipes = pd.DataFrame(lines[idx_pipes[0]:idx_pipes[1]], columns=cols)
			df_pipes[['lon','lon1','lon2']] = df_pipes[['lon','lon1','lon2']].apply(pd.to_numeric)
			df_pipes['pred'] = df_pipes['pred'].astype(str)
			df_pipes['succ'] = df_pipes['succ'].astype(str)
			df_pipes = df_pipes.sort_values(by=['succ', 'pred']).reset_index(drop=True)
			
			# ================== DIAMETRES (TD) ==================
			# Récupération des diamètres intérieurs des tuyaux disponibles
			cols = ['ID','Nominal','SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness', 'N']
			df_diameters = pd.DataFrame(lines[idx_diameters[0]:idx_diameters[1]], columns=cols)
			df_diameters[['SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness']] = \
				df_diameters[['SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness']].apply(pd.to_numeric)
			df_diameters['ID'] = df_diameters['ID'].astype(str)

			# ================== PARAMETRES (TP) ==================
			# Récupération des paramètres de la simulation et du réseau
			default_params = {
				"bescal": 8.9396e-04,
				"bounded_flow": 0,
				"faucetcoef": 1.0,
				"fusion": 0,
				"lambda": 1.7810,
				"limitbudget": float('inf'),
				"nbsim": 100,
				"nu": 4.7810,
				"opentaps": 1,
				"orif_com": 1,
				"orifcoef": 1.0,
				"pipelength": 6.0,
				"pressure": 2,
				"secu": 1.0,
				"servicequal": 0.65,
				"seuil": 0.0,
				"split": 0,
				"targetflow": 0.0,
				"uniform_dem": 1,
				"use_efficient_pipes": 0,
				"vmax": 1.5,
				"vmax_secu": 0.95,
				"watertemp": 20.0,
			}

			params = {line[0]: float(line[1]) for line in lines[idx_prop[0]:idx_prop[1]]}
			param = {**default_params, **params}

			# ================== CLIENTS (TC) ==================
			# Récupération des clients du réseau, de leur demande et de leur probabilité d'ouverture
			cols = ['client','borne','proba','cible']
			df_clients = pd.DataFrame(lines[idx_client[0]:idx_client[1]], columns=cols)
			df_clients[['proba','cible']] = df_clients[['proba','cible']].apply(pd.to_numeric)
			df_clients['client'] = df_clients['client'].astype(str)
			df_clients['borne'] = df_clients['borne'].astype(str)
			
			if df_clients['proba'].nunique() != 1 or df_clients['cible'].nunique() != 1:
				param['uniform_dem'] = 0
				param['bounded_flow'] = 1
			else:
				param['uniform_dem'] = 1

			# ================== BORNES (TB) ==================
			# Informations sur les bornes 
			cols = ['borne','pressure', 'N']
			maskTerm = df_nodes['statut'] == 2
			terminal = df_nodes.loc[maskTerm, 'noeud'].astype(str)
			
			if idx_bornes is not None:
				df_bornes = pd.DataFrame(lines[idx_bornes[0]:idx_bornes[1]], columns=cols)
				df_bornes['pressure'] = pd.to_numeric(df_bornes['pressure'])
				df_bornes['borne'] = df_bornes['borne'].astype(str)
				df_bornes = df_bornes.sort_values(by='borne').reset_index(drop=True)
				
				if set(df_bornes['borne'].unique()) != set(terminal):
					raise ValueError("Les bornes ne correspondent pas aux noeuds terminaux")
			else:
				df_bornes = pd.DataFrame({
					'borne': terminal,
					'pressure': param['pressure']
				}).sort_values(by='borne').reset_index(drop=True)
				
		return df_nodes, df_pipes, df_clients, df_bornes, df_diameters, param
		
	def completeTables(self):
		"""
		Complete the tables with missing values and calculate derived parameters.
		Steps:
		1- Add successor height (gravite)
		2- Map Diameter/Cost for ID1/ID2 if present
		3- Calculate beta coefficient per segment
		4- Ensure TA.cost if absent (fallback: minimal cost per diameter)
		5- Place bornes and sources. Arc status.
		6- Calculate alpha parameters."""
		#  ==== 1- Ajout de la hauteur du successeur ======
		# self.pipes['haut_succ'] = self.pipes['succ'].map(self.nodes.set_index('noeud')['haut'])
		self.pipes['gravit'] = 0.0
		
		source = self.nodes['statut'] == 0
		map_source = self.nodes[source].set_index('noeud')['haut']
		mask_source = self.pipes['pred'].isin(map_source.index)
		self.pipes.loc[mask_source, 'gravit'] -= self.pipes.loc[mask_source, 'pred'].map(map_source)
	
		borne = self.nodes['statut'] == 2
		map_borne = self.nodes[borne].set_index('noeud')['haut']
		mask_borne = self.pipes['succ'].isin(map_borne.index)
		self.pipes.loc[mask_borne, 'gravit'] += self.pipes.loc[mask_borne, 'succ'].map(map_borne)

		# ===== 2- Mappage Diameter/Cost pour ID1/ID2 si présents ======
		needTD = set(['diam', 'cost']).issubset(self.diameters.columns)
		assert needTD, "TD doit contenir les colonnes 'diam' et 'cost'."

		self.diameters['ID'] = self.diameters['ID'].astype(str)

		#  Assurer TA.diam (bloquant si impossible)
		hasID    = 'ID1' in self.pipes.columns and 'ID2' in self.pipes.columns
		hasDiam  = 'diam1' in self.pipes.columns and 'diam2' in self.pipes.columns
		if not hasDiam:
			if hasID:
				# ID -> Diameter via TD (join « left », ordre de TA préservé)
				self.pipes['diam1'], self.pipes['cost1']\
					=  map_diam_cost(self.pipes['ID1'], self.pipes['lon1'], self.diameters, 10, 0)
				self.pipes['diam2'], self.pipes['cost2']\
					=  map_diam_cost(self.pipes['ID2'], self.pipes['lon2'], self.diameters, 10, 0)
			else:
				raise ValueError("Diamètres manquants: ni TA.diam présent, ni TA.ID permettant \
				de le reconstituer. Impossible de simuler les débits.")
		# Vérifier qu'on a bien obtenu des diamètres utilisables
		if not np.isfinite(self.pipes[['diam1', 'diam2']]).to_numpy().all():
			raise ValueError('Diamètres manquants/invalides après normalisation.\
					Vérifier la cohérence TA.ID ↔ TD.ID/TD.diam.')
		
		# === 3- Coefficient beta par tronçon ===
		# S'assurer que chaque segment a une longueur positive
		lon = self.pipes['lon1'] + self.pipes['lon2']
		fautif = lon == 0
		if fautif.any():
			raise ValueError(f"L'arc ayant pour successeur {self.pipes.loc[fautif, 'succ'].iloc[0]}\
					a une longueur nulle\n")
		self.pipes['beta'] = 0.0
		idlon1 = self.pipes['lon1'] > 0
		idlon2 = self.pipes['lon2'] > 0

		self.pipes.loc[idlon1, 'beta'] = self.param['bescal'] * \
			self.pipes.loc[idlon1, 'lon1'] / (self.pipes.loc[idlon1, 'diam1'] ** self.param['nu'])

		self.pipes.loc[idlon2, 'beta'] = self.pipes.loc[idlon2, 'beta'] + \
			self.param['bescal'] * self.pipes.loc[idlon2, 'lon2'] / \
				(self.pipes.loc[idlon2, 'diam2'] ** self.param['nu'])
		
		# === 4- Assurer TA.cost si absent (fallback: coût minimal par diamètre) ===
		hasCost = 'cost1' in self.pipes.columns or 'cost2' in self.pipes.columns
		if not hasCost:
			# Construire un mapping Diameter -> min(Cost)
			# (si plusieurs lignes partagent le même diamètre, on prend le coût min)
			if self.diameters['diam'].duplicated().any():
				print("At least one diameter appears twice. The least cost is retained.")
			min_cost = self.diameters.groupby('diam')['cost'].min()
			self.pipes['cost1'] = self.pipes['diam1'].map(min_cost)
			self.pipes['cost2'] = 0.0
			mask = self.pipes['lon2'] > 0
			self.pipes.loc[mask, 'cost2'] = self.pipes.loc[mask, 'diam2'].map(min_cost)

		# --- Coût total par tronçon ---
		if 'cost1' in self.pipes.columns and 'cost2' in self.pipes.columns:
			self.pipes['total_cost'] = self.pipes['lon1'] * self.pipes['cost1'] +\
				self.pipes['lon2'] * self.pipes['cost2'] 
		
		# ===== 5- Placer les bornes et les sources. Statut des arcs ======
		self.pipes['statut'] = 1
		idter = self.nodes['statut'] == 2
		idsrc = self.nodes['statut'] == 0
		nodes_ter = self.nodes.loc[idter, 'noeud']
		nodes_src = self.nodes.loc[idsrc, 'noeud']
		idpred = self.pipes['pred'].isin(nodes_ter)
		idsucc = self.pipes['succ'].isin(nodes_ter)
		if idpred.any():
			raise ValueError("Il y a des bornes parmi les prédécesseurs")
		else:
			self.pipes.loc[idsucc, 'statut'] = 2
		idsucc = self.pipes['succ'].isin(nodes_src)
		idpred = self.pipes['pred'].isin(nodes_src)
		if idsucc.any():
			raise ValueError("Il y a des sources parmi les successeurs")
		else:
			self.pipes.loc[idpred, 'statut'] = 0
		
		# ==== 6- Calculer les paramètres alpha ====
		id_orifid = 'orif_ideal' in self.nodes.columns
		id_orifcom = 'orif_com' in self.nodes.columns
		id_ecart = 'ecart' in self.pipes.columns
		alpha_faucet = 1 / self.param['faucetcoef'] if self.param['faucetcoef'] > 0 else 0.0
		loc = pd.Index(self.nodes['noeud']).get_indexer(self.pipes['succ'])
		self.pipes['alpha_ideal'] = 0.0
		self.pipes['alpha_com'] = 0.0
		idBorne = self.pipes['statut'] == 2
		if id_orifid:
			cible = self.param['targetflow']/1000
			self.pipes['orif_ideal'] = self.nodes.iloc[loc]['orif_ideal'].to_numpy()
			self.pipes.loc[idBorne, 'alpha_ideal'] = (self.param['orifcoef'] /
				self.pipes.loc[idBorne,'orif_ideal'])**4 + alpha_faucet
		if id_orifcom:
			self.pipes['orif_com'] = self.nodes.iloc[loc]['orif_com'].to_numpy()
			self.pipes.loc[idBorne, 'alpha_com'] = (self.param['orifcoef'] /
				self.pipes.loc[idBorne,'orif_com'])**4 + alpha_faucet
		if id_ecart:
			self.pipes['alpha'] = self.pipes['ecart']/cible**2
		else:
			self.pipes['alpha'] = self.pipes['alpha_ideal']
		
		# Tris des dataFrames
		self.nodes = self.nodes.sort_values(by=['statut', 'noeud']).reset_index(drop=True)
		self.pipes = self.pipes.sort_values(by=['statut', 'succ']).reset_index(drop=True)
	
	def complete_data(self):
		"""
		Calcule la matrice d'oncidence si elle n'est pas présente dans le champ de
		la structure dat. Définit la structure des paramètres par défaut de
		l'algorithme IPM. Enrichit dat de champs vecteurs indicateurs des
		positions des noeuds et des arcs
		"""
		# ---- incidence matrix ------
		A = self.incidence_intermediaire()

		self.dat = {
			# ---- Sauvegarder le coût total d'investissement ----
			"totcost" : sum(self.pipes['total_cost']),
			# ---- Générer les indicateurs ----
			"idNter"  : (self.nodes['statut'] == 2).to_numpy(),
			"idNtra"  : (self.nodes['statut'] == 1).to_numpy(),
			"idNsrc"  : (self.nodes['statut'] == 0).to_numpy(),
			"idAter"  : (self.pipes['statut'] == 2).to_numpy(),
			"idAtra"  : (self.pipes['statut'] == 1).to_numpy(),
			"idAsrc"  : (self.pipes['statut'] == 0).to_numpy(),
			"nbA"     : len(self.pipes),
			"M_incid" : A
		}
		# Dénombrement des diverses catégories.
		self.dat['nbNter'], self.dat['nbNtra'], self.dat['nbNsrc']\
			= np.sum(self.dat['idNter']), np.sum(self.dat['idNtra']), np.sum(self.dat['idNsrc'])
		self.dat['nbAter'], self.dat['nbAtra'], self.dat['nbAsrc']\
			= np.sum(self.dat['idAter']), np.sum(self.dat['idAtra']), np.sum(self.dat['idAsrc'])

		self.dat['gravite'] = self.pipes['gravit'].to_numpy()
		self.dat['beta'] = self.pipes['beta'].to_numpy()
		self.dat['alpha'] = self.pipes['alpha'].to_numpy()

	def init_options(self):
		"""--- Options pour le code IPM ---"""
		return {"tolx": 1e-8,       # Soit 1 millilitre par seconde
				"tolpress": 1e-6,	# Soit la pression de 1mm d'eau (1e-4 bar)
				"tolpseudoA": 1e-10,# Tolerance de déclaration pseudo actif
				"maxiter": 30,      # Nombre maximum d'itérations pour la méthode de Newton
				"tolcent": 1e-10,   # paramètre pour la régularisation en y
				"sigma": 0.1,       # reduction factor for mu (kept as in your code)
				"tau": 2.5,         # exponent used in sigma_mu = min(1,(mu_aff/mu)^tau)*mu
				"a_secur": 0.999}   # safeguard on final step
		
	def incidence_intermediaire(self):
		"""
		Calcule la matrice d'incidence du graphe à partir des tables de noeuds et d'arcs.
		La matrice d'incidence est construite en considérant uniquement les noeuds de degré >= 2,
		et en orientant les arcs de pred vers succ. Les arcs connectant des noeuds de degré 1 sont ignorés,
		car ils ne contribuent pas à la conservation du flux dans les noeuds de transit.
		La matrice est retournée au format sparse CSR pour une utilisation efficace dans les calculs ultérieurs.
		"""
		allEnds = pd.concat([self.pipes['pred'], self.pipes['succ']], ignore_index=True)
		self.nodes['degre'] = self.nodes['noeud'].map(allEnds.value_counts(sort=False)).fillna(0).astype(int)
		nodes_rows = self.nodes[self.nodes['degre'] >= 2]['noeud']
		nodes_idx = pd.Index(nodes_rows)
		
		m, n = len(nodes_rows), len(self.pipes)
		p_idx, s_idx = nodes_idx.get_indexer(self.pipes['pred']), nodes_idx.get_indexer(self.pipes['succ'])
		mask_p, mask_s = p_idx != -1, s_idx != -1

		P = sparse.coo_matrix((np.ones(mask_p.sum()), (p_idx[mask_p], np.flatnonzero(mask_p))), shape=(m, n))
		S = sparse.coo_matrix((np.ones(mask_s.sum()), (s_idx[mask_s], np.flatnonzero(mask_s))), shape=(m, n))
		return (S - P).tocsr()
	
	def review_simul_parameters(self):
		if self.param['uniform_dem'] == 0:
			self.param['bounded_flow'] = 1
			wanted = ["nbsim","pressure"]
		else:
			wanted = ["nbsim","bounded_flow","opentaps","targetflow","orif_com"]

		print_flag = False
		if self.param['uniform_dem']==1 and self.param['bounded_flow']==1:
			wanted.append("pressure")
			print_flag = True
		
		param_wanted = {key: self.param.get(key) for key in wanted}
		for key in param_wanted.keys():
			print(f"Do you want to change {key} ? Current value: {self.param[key]}.\
				Put a new value or press ENTER to skip.")
			answer = input()	
			if answer != "":
				try:
					if key in ['opentaps', 'targetflow']:
						value = float(answer)
					else:
						value = int(answer)
					self.param[key] = value
					print(f"New value: {value}")
				except ValueError:
					print("Invalid input, value not changed.")
			else:
				continue
		if (self.param['uniform_dem']==1 and self.param['bounded_flow']==1) and not print_flag:
			print(f"Since uniform_dem=1 and bounded_flow=1, pressure is required.\
				Current value: {self.param['pressure']}. Put a new value or press ENTER to skip.")
			answer = input()	
			if answer != "":
				try:
					value = float(answer)
					self.param['pressure'] = value
					print(f"New value: {value}")
				except ValueError:
					print("Invalid input, value not changed.")
