import warnings
import pandas as pd
import numpy as np
import os
from pathlib import Path
from scipy import sparse
from scipy.stats import binom
from utilsDesign import*

class DesignData:
	def __init__(self, name, version, output):
		self.name = name
		self.version = version
		self.output = output
		self.nodes, self.pipes, self.client, self.borne,\
			self.diameters, self.orifices, self.param = self.parse_tpo_to_df()
		self.context = self.build_context()
	def __str__(self):
		return (f"Nom de l'instance: {self.name}\n"
				f"Version: {self.version}\n"
				f"Sortie: {self.output}")

	def get_path(self, sub_dir):
		ROOT_DIR = Path(__file__).resolve().parent.parent.parent
		SUB_DIR = ROOT_DIR / sub_dir / self.name
		return SUB_DIR
	
	def parse_tpo_to_df(self):
		tpo_path = self.get_path("examples/tpo_examples")
		tpo_file = os.path.join(tpo_path, f"{self.name}{self.version}.tpo")
		if not os.path.exists(tpo_file):
			raise FileNotFoundError(f"Fichier introuvable : {tpo_file}")
		
		has_client = False
		has_borne  = False
		has_diameter = False
		has_orifice = False

		with open(tpo_file, "r") as file:
			raw_lines = [line.strip() for line in file.readlines()]

			idx_client    = get_section_indices(raw_lines, "Clients")
			idx_nodes     = get_section_indices(raw_lines, "Nodes")
			idx_pipes     = get_section_indices(raw_lines, "Pipes")
			idx_prop      = get_section_indices(raw_lines, "Default properties")
			idx_diameters = get_section_indices(raw_lines, "Diameters")
			idx_bornes    = get_section_indices(raw_lines, "Borne")
			idx_orifices   = get_section_indices(raw_lines, "Orifices")

			missing = []
			if idx_nodes is None: missing.append("Nodes")
			if idx_pipes is None: missing.append("Pipes")
			if idx_prop is None: missing.append("Default properties")

			if missing:
				raise ValueError(f"Section(s) manquante(s) : {', '.join(missing)}")
			lines = [[val.strip() for val in line.split(',')] for line in raw_lines]

			# ================== NOEUDS (TN) ==================
			cols = ['noeud','haut','xdata','ydata','nbrob', 'statut']
			df_nodes = pd.DataFrame(lines[idx_nodes[0]:idx_nodes[1]], columns=cols)
			df_nodes[['haut','xdata','ydata','nbrob', 'statut']] \
				= df_nodes[['haut','xdata','ydata','nbrob', 'statut']].apply(pd.to_numeric)
			df_nodes['noeud'] = df_nodes['noeud'].astype(str)
			df_nodes = df_nodes.sort_values(by=['statut', 'noeud']).reset_index(drop=True)

			# ================== ARCS (TA) ==================
			cols = ['pred','succ','lon']
			df_pipes = pd.DataFrame(lines[idx_pipes[0]:idx_pipes[1]], columns=cols)
			df_pipes['lon'] = pd.to_numeric(df_pipes['lon'])
			df_pipes['pred'] = df_pipes['pred'].astype(str)
			df_pipes['succ'] = df_pipes['succ'].astype(str)
			df_pipes = df_pipes.sort_values(by=['succ', 'pred']).reset_index(drop=True)
			# Zero-length arcs are interpreted as control devices (valves, orifices).
			# Since the current hydraulic model assumes a geometric pipe segment,
			# we replace zero lengths by a minimal artificial length.
			# Future versions may treat such elements explicitly.
			lon_eps = 0.01   # minimal artificial length (m)
			df_pipes['lon'] = df_pipes['lon'].apply(lambda x: max(x, lon_eps))

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
			# Note (NW4 compatibility)
			# ------------------------
			# The parameter 'limitbudget' is kept only for backward compatibility with
			# the NW4 Java implementation. It is not used in the NW5 design algorithm.
			# This parameter may disappear in the future Python implementation.
			# The same for 'seuil' and 'watertemp'. Other parameters may be necessary
			# in the future concerning the formula for friction in pipes.

			# =========== Read or build section Client (TC) ===========
			if idx_client is not None: has_client=True
			cols = ['client','borne','proba','cible']
			if has_client: # if clients exists in the tpo file
				df_client = pd.DataFrame(lines[idx_client[0]:idx_client[1]], columns=cols)
				df_client[['proba','cible']] = df_client[['proba','cible']].apply(pd.to_numeric)
				a = df_client['proba'].nunique()
				b = df_client['cible'].nunique()
				if a!=1 or b!=1:
					param['uniform_dem']=0
					param['bounded_flow']=1
				else:
					param['uniform_dem']=1
			else:
				client_data = []
				cpt = 0
				for i in range(len(df_nodes)):
					if df_nodes['statut'].iloc[i]!=2 or df_nodes['nbrob'].iloc[i]<=0:continue
					b = str(df_nodes['noeud'].iloc[i])
					for k in range(int(df_nodes['nbrob'].iloc[i])):
						cpt += 1
						client_data.append({'df_client':f"C{k}{b}", 'borne':b, \
						'proba':param['opentaps'], 'cible':param['targetflow']})
				df_client = pd.DataFrame(client_data, columns=cols)
				param['uniform_dem'] = 1
			
			# =========== Read or build section Borne (TB) ===========
			if idx_bornes is not None: has_borne=True
			terminal = df_nodes.loc[df_nodes['statut'] == 2, 'noeud']
			if has_borne:
				cols = ['borne','pressure','N']
				df_borne = pd.DataFrame(lines[idx_bornes[0]:idx_bornes[1]], columns=cols)
				unique_borne = df_borne['borne'].unique()
				hasSameSize = len(unique_borne)==len(terminal)
				areEqual = sorted(unique_borne)==sorted(terminal)
				if not areEqual:
					raise ValueError(f"Les bornes ne correspondent pas aux noeuds terminaux")
			else:
				cols = ['borne','pressure']
				df_borne = pd.DataFrame()
				df_borne['borne'] = terminal
				df_borne['pressure'] = param['pressure']
			df_borne = df_borne.sort_values('borne').reset_index(drop=True)
			df_borne['pressure'] = pd.to_numeric(df_borne['pressure'])

			# ======= DIAMÈTRES (TD) in the file or separately =========
			if idx_diameters is not None: has_diameter=True
			if has_diameter:
				cols = ['size','SDR','diam','cost','pressure','type','rugo']
				df_diameters = pd.DataFrame(lines[idx_diameters[0]:idx_diameters[1]], columns=cols)
			else:
				df_diameters = self.parse_diameters()
			df_diameters = df_diameters.sort_values(by='diam').reset_index(drop=True)
			df_diameters[['diam','cost','pressure','type','rugo']] = \
					df_diameters[['diam','cost','pressure','type','rugo']].apply(pd.to_numeric)
			# ID D001, D002,... en première colonne
			n = len(df_diameters)
			IDs = [f"D{i+1:03d}" for i in range(n)]
			df_diameters.insert(0, 'ID', IDs)

			# ================ ORIFICES (TO) =================
			if idx_orifices is not None: has_orifice=True
			if has_orifice:
				cols = ['in_diam', 'N']
				df_orifices = pd.DataFrame(lines[idx_orifices[0]:idx_orifices[1]], columns=cols)
				df_orifices['in_diam'] = pd.to_numeric(df_orifices['in_diam'])
			else:
				df_orifices = self.parse_orifices()

		return df_nodes, df_pipes, df_client, df_borne, df_diameters, df_orifices, param

	def parse_diameters(self):
		diam_path = self.get_path("examples/tpo_examples")
		diam_file = os.path.join(diam_path, f"DiamBase{self.name}.db")
		if not os.path.exists(diam_file):
			raise FileNotFoundError(f"Fichier introuvable : {diam_file}")
		lines = []
		with open(diam_file, "r") as file:
			lines = [line.strip().split(',') for line in file.readlines()]
		cols = ['size','SDR','diam','cost','pressure','type','rugo']
		diameters = pd.DataFrame(lines, columns=cols)
		return diameters
		
	def parse_orifices(self):
		ori_path = self.get_path("examples/tpo_examples")
		ori_file = os.path.join(ori_path, f"orifices.db")
		if not os.path.exists(ori_file):
			raise FileNotFoundError(f"Fichier introuvable : {ori_file}")
		cols = ['in_diam']
		orifices = []
		with open(ori_file, "r") as file:
			orifices = pd.DataFrame([line.strip().split(',') for line in file], columns=cols)
			orifices['in_diam'] = pd.to_numeric(orifices['in_diam'])
		return orifices

	def validate_and_align(self):
		"""Objectif: Nettoyer et valider TN/TA immédiatement après parseTpoToTable.
		Hypothèses de nommage: TN.noeud (categorical), TA.pred/TA.succ (categorical).
		Effets:
			- interdit nœuds inconnus
			- vérifie unicité des nœuds TN
			- vérifie égalité ensembliste entre nœuds(TA) et nœuds(TN)
			- aligne les catégories de TA sur celles de TN
			- (optionnel) repère self-loops et multi-arêtes
		"""
		# 0) Assurer le type
		all_nodes_list = self.nodes['noeud'].unique().tolist()
		cat_type = pd.CategoricalDtype(categories=all_nodes_list)
		self.nodes['noeud'] = self.nodes['noeud'].astype(cat_type)
		self.pipes['pred'] = self.pipes['pred'].astype(cat_type)
		self.pipes['succ'] = self.pipes['succ'].astype(cat_type)
		self.borne['borne'] = self.borne['borne'].astype(cat_type)

		# 1) Référentiel EXACT des nœuds (sans unique, qui réordonne)
		idsTN   = set(self.nodes['noeud'])
		idsPred = set(self.pipes['pred'])
		idsSucc = set(self.pipes['succ'])
		idsBorne = set(self.borne['borne'])

		# 2) Contrôles d'ensemble (double inclusion) avec messages clairs
		badPred = idsPred - idsTN
		badSucc = idsSucc - idsTN
		badBorneN = idsBorne - idsTN
		badBorneS = idsBorne - idsSucc
		if badPred or badSucc:
			raise ValueError(f"Noeuds inconnus dans TA: pred={badPred} succ={badSucc}")
		missingInTA = idsTN - (idsPred | idsSucc)
		if missingInTA:
			raise ValueError(f"Noeuds présents dans TN mais absents de TA: {missingInTA}")
		if badBorneN or badBorneS:
			raise ValueError(f"Noeuds inconnus dans TB: borne={badBorneN} ou {badBorneS}")
		missingInTA = idsBorne - (idsPred | idsSucc)
		if missingInTA:
			raise ValueError(f"Noeuds présents dans TB mais absents de TA: {missingInTA}")

		# 3) (Optionnel) quelques garde-fous utiles
		#    - self-loops
		hasSelf = np.any(self.pipes['pred'] == self.pipes['succ'])
		if hasSelf:
			idx = self.pipes.index[self.pipes['pred'] == self.pipes['succ']].tolist()
			i = idx[0] if idx else None
			raise ValueError(f"Self-loop détecté dans les arcs. sur la ligne {i}")
		#   - multi-arêtes (même pred/succ répété)
		pairs = self.pipes[['pred', 'succ']]
		mult = pairs.duplicated(keep=False)

		if mult.any():
			n_multi = pairs[mult].drop_duplicates().shape[0]
			print(f"{n_multi} paires pred/succ en multi-arêtes détectées.")

	def peel_orient_tree(self):
		"""
		PEEL_ORIENT
			Oriente les arcs d'un réseau ramifié en "pelant" les feuilles (≠ source)
			jusqu'à stabilisation, détecte arbre vs maille, et restitue TA orienté.
		
		Hypothèses :
			- La source est l'unique nœud avec TN.statut == 0 et a degré 1 (sinon error).
			- TN.noeud, TA.pred, TA.succ peuvent être string, char, ou categorical.
			- On NE réordonne NI TN NI TA ; on préserve toutes les colonnes métier.
		
		Entrées :
			TN : table nœuds, doit contenir au moins TN.noeud (id), TN.statut (0=source)
			TA : table arcs, doit contenir au moins TA.pred (id amont), TA.succ (id aval)
		
		Sorties :
			TA_oriented : copie de TA avec colonnes pred/succ réécrites (orientation corrigée),
						+ colonnes auxiliaires pred_idx/succ_idx (indices 1..n dans TN)
			TN_out      : TN (inchangé)
			report      : struct (iterations, layers, leftoverTA, is_tree,
			has_cycle, reason) 
		"""
		# -------- 0) Préparations & validations légères
		n = self.nodes.shape[0]
		m = self.pipes.shape[0]
		if 'noeud' not in self.nodes.columns:
			raise ValueError('TN doit contenir la variable "noeud".')

		if 'statut' not in self.nodes.columns:
			raise ValueError('TN doit contenir la variable "statut" (0 = source).')

		if not all(col in self.pipes.columns for col in ['pred', 'succ']):
			raise ValueError('TA doit contenir les variables "pred" et "succ".')

		# On compare par libellés : passer tout en string pour un mapping robuste
		nodeLabels = self.nodes['noeud'].astype("string")
		predLabels = self.pipes['pred'].astype("string")
		succLabels = self.pipes['succ'].astype("string")
		# Source : ligne (row) puis code stable = index de ligne
		idsource = self.nodes['statut'] == 0
		if idsource.sum() != 1:
			raise ValueError(f"Il doit y avoir exactement une source avec TN.statut == 0 (trouvé: {idsource.sum()}).")
		srcRow = np.where(idsource)[0][0]

		# -------- 1) Mapping TA → indices 0...n-1 (positions dans TN)
		node_index = pd.Index(nodeLabels)
		pred_idx = node_index.get_indexer(predLabels)
		succ_idx = node_index.get_indexer(succLabels)
		tfp = pred_idx != -1
		tfs = succ_idx != -1

		if not (tfp.all() and tfs.all()):
			missPred = np.unique(np.array(predLabels)[~tfp])
			missSucc = np.unique(np.array(succLabels)[~tfs])
			raise ValueError(f"Noeuds d'arc inconnus: pred manquants={','.join(missPred)}\
							| succ manquants={','.join(missSucc)}")

		# -------- 2) Masques actifs (sous-réseau dynamique)
		active_nodes = np.ones(n, dtype=bool)
		active_edges = np.ones(m, dtype=bool)

		# -------- 3) Vérifier hypothèse : degré(source) == 1
		degCounts = degree_counts_simple(pred_idx, succ_idx, active_edges, active_nodes, n)

		if degCounts[srcRow] != 1:
			msg = (	f"Hypothèse violée : la source n'est pas de degré 1 (deg={degCounts[srcRow]})\n"
					"Ajouter une source fictive, à 0.1m de la source et avec la même altitude\n"
					"et diminuer l'altitude de la source originale de 1m.")
			print(msg)
			raise ValueError("Bon travail d'ajustement du réseau")

		# -------- 4) Boucle de pelage (while : tant qu'il existe une feuille ≠ source)
		peeled_layers = []
		iter = 0
		maxIter = n + 1

		while True:
			iter += 1
			if iter > maxIter:
				break
			
			degCounts = degree_counts_simple(pred_idx, succ_idx, active_edges, active_nodes, n)
			# feuilles = nœuds actifs de degré 1, hors source
			leaves = np.where(active_nodes & (degCounts == 1))[0]
			leaves = leaves[leaves != srcRow]

			if leaves.size == 0: # STOP clair : plus de feuille ≠ source
				break
			
			# Ré-orienter : tout arc incident à une feuille doit avoir la feuille en succ
			isPredLeaf = active_edges & np.isin(pred_idx, leaves)
			isSuccLeaf = active_edges & np.isin(succ_idx, leaves)
			maskSwap = isPredLeaf & ~isSuccLeaf # seule l'extrémité pred est feuille

			# Peler : désactiver arcs incidents + nœuds feuilles
			if np.any(maskSwap):
				pred_idx[maskSwap], succ_idx[maskSwap] = succ_idx[maskSwap], pred_idx[maskSwap]
			
			incident = active_edges & (isPredLeaf | isSuccLeaf)
			active_edges[incident] = False
			active_nodes[leaves] = False

			# Log lisible (labels des feuilles pelées)
			peeled_layers.append(self.nodes['noeud'].iloc[leaves].to_numpy())
		
		# -------- 5) Diagnostic final (arbre vs maille)
		left_m = np.sum(active_edges)

		if left_m == 1:
			i = np.where(active_edges)[0][0]
			one_arc_to_source = (pred_idx[i] == srcRow) or (succ_idx[i] == srcRow)
		else:
			one_arc_to_source = False
		is_tree = (left_m == 0) or one_arc_to_source
		has_cycle = not (is_tree and (left_m >= 2))

		# Sous-réseau restant (utile si maille)
		leftoverTA = self.pipes.iloc[active_edges]

		# -------- 7) Rapport final
		if is_tree:
			if left_m == 0:
				reason = "Sous-réseau épuisé: aucun arc restant."
			else:
				reason = "Un seul arc restant incident à la source."
		else:
			reason = f"Stoppé sans feuille ≠ source, {left_m} arcs restants : maille."
		report = {
			"iterations": iter - 1,
			"layers": peeled_layers,
			"leftoverTA": leftoverTA,
			"is_tree": is_tree,
			"has_cycle": has_cycle,
			"reason": reason
		}
		return report

	#TO DO
	#def review_design_parameters(self):

	def classify_case(self):
		"""
		Test whether the situation is with identical users, or with one user per
		terminal, or with multiple users per terminal and a constraint on
		pressure and flow. (Multiple users at the same terminal are all at the
		same height.)
		The case that is not treated in makeDesign is when there are multiple
		heterogeneous users at a terminal (borne) and no constraint on the flows.
		This case requires a redesign of the topography with fictitious arcs or
		the addition an orifice à the terminal and not at the individual faucets.
		If all faucets linked to the terminal have identical equipment, it is
		still possible to propose a design. (Just substract to the elevation the
		the headloss contribution of a single representative faucet.)
		"""

		isOnePerBorne = not self.client['borne'].duplicated().any()
		isNW4 = (self.client['proba'].nunique() == 1) and (self.client['cible'].nunique() == 1)
		classCase = {'sim':
					{'caseType': None,
					'alert': ""}
					}
		if isNW4:
			classCase['sim']['caseType'] = 1
		elif isOnePerBorne:
			classCase['sim']['caseType'] = 2
		else:
			if self.param['pressure'] or self.param['bounded_flow']:
				classCase['sim']['caseType'] = 31
			else:
				classCase['sim']['caseType'] = 32
				classCase['sim']['alert'] = "Plusieurs usagers par borne sans contrainte P/Q : ramener à Cas 2 (arcs fictifs)."
				warnings.warn("Plusieurs usagers par borne sans contrainte P/Q : ramener à Cas 2 (arcs fictifs).")
		return classCase

	def enrich_tables(self):
		"""
		ENRICHTABLES: Ajoute aux arcs les infos utiles et dérivées.
		Entrée: TA(pred,succ,lon,...)  TN(noeud,haut,...)  TD(...)
		Sortie: TAe = TA enrichie (ex: pmax), TNe =TN (ex: degree). TN/TD 
		éventuellement typées.
		"""
		# Check class of variables 'noeud', 'pred', 'succ'
		assert(self.nodes['noeud'].dtype == 'category'), 'The list of nodes must be categorical (see parseTpoToTables).'
		assert(self.pipes['pred'].dtype == 'category' and self.pipes['succ'].dtype == 'category'),\
			'TA.pred/TA.succ must be categorical (see parseTpoToTables).'
		
		# === Calcul de la pression maximale sur chaque arc ===
		map_haut = self.nodes.set_index('noeud')['haut']
		# Hauteur du prédécesseur
		haut_pred = self.pipes['pred'].map(map_haut).astype(float)
		# Hauteur du successeur
		haut_succ = self.pipes['succ'].map(map_haut).astype(float)

		self.pipes['pmax'] = -np.minimum(haut_pred, haut_succ)

		# === Calcul du degré des noeuds ===
		counts = pd.concat([self.pipes['pred'], self.pipes['succ']]).value_counts()
		self.nodes['degree'] = self.nodes['noeud'].map(counts).fillna(0).astype(int)

		# === Add the variables haut,statut and nrob to TA === 
		TNsucc = self.nodes[['noeud', 'haut', 'statut', 'nbrob']].rename(columns={'noeud': 'succ'})
		self.pipes = self.pipes.merge(TNsucc, on='succ', how='left')

		# === Add the variable pression to TA === 
		# Pressure at junction (borne)
		map_pressure = self.borne.set_index('borne')['pressure']
		pressure = self.pipes['succ'].map(map_pressure).fillna(0).to_numpy()
		self.pipes['pressure'] = pressure

		# ===== Give statut=0 to the arc adjacent to the source =====
		src = self.nodes.loc[self.nodes['statut'] == 0, 'noeud']
		a = self.pipes['pred'].isin(src)
		self.pipes.loc[a, 'statut'] = 0

		# ==== Reorder TA just like TN (done in parseTpoToTable) ====
		self.pipes = self.pipes.sort_values(by=['statut', 'succ']).reset_index(drop=True)

	def build_context(self):
		"""
		buildContext  Initialise la structure de contexte pour CreateDsg / Fusion
		
		Input :
			param : struct des paramètres (doit contenir le champ 'fusion')
		
		Output :
			ctx    : struct contexte d'exécution
					- useFusion   : booléen, option demandée par l'utilisateur
					- changed     : booléen, indique si des fusions ont été faites
					- TableFusion : table des fusions (vide par défaut)
					- log         : cell array pour des messages éventuels
		"""
		context = {
			'useFusion': bool(self.param['fusion']),
			'changed': False,
			'TableFusion': [],
			'log': [],
			'split': bool(self.param['split'])
		}
		return context


	def fusion_nodes_degre_2(self):
		"""
		FUSION This function concatenates the two arcs that are adjacent to the same 
		node of degree 2 into a single arc whose length is the sum of the two lengths.
		Input: tables of arcs, nodes and diameters
		Output: table of arcs and nodes after merger. Table of merged arcs.
		Fusion is not allowed if the adjacent arcs to degree two node have access
		to the same set of allowed pipes. The concatenated arc is endow with the
		largest pmax of its two members.
		"""
		# Si la fusion n'est pas demandée, on ressort de la fonction
		if not self.context['useFusion']:
			return 0
		
		# (a) Règle pression (si données disponibles)
		# autorisé si pmax(a) <= pressure(d)
		allowedPipes = buildAllowedPipes(self.pipes, self.diameters)

		TableNoeudsFusion = self.nodes.copy()
		TableArcsFusion = self.pipes.copy()
		arcs_records = TableArcsFusion.to_dict('index')
		nodes_records = TableNoeudsFusion.set_index('noeud').to_dict('index')

		# Création de listes d'adjacence pour les arcs (successeurs et prédécesseurs de chaque noeud)
		succ_map = {}
		pred_map = {}
		for idx, arc in arcs_records.items():
			succ_map.setdefault(arc['succ'], []).append(idx)
			pred_map.setdefault(arc['pred'], []).append(idx)

		# Ensembles des arcs et noeuds à supprimer
		arcs_to_delete = set()
		nodes_to_delete = set()
		new_arcs_counter = len(arcs_records)

		allowed_dict = {i: allowedPipes[i, :] for i in range(len(allowedPipes))}

		for ni, n_data in nodes_records.items():
			# On ne fusionne que les noeuds de degré 2 et qui ne sont pas des sources
			if n_data['degree'] != 2 or n_data['statut'] == 0:
				continue
			if ni in nodes_to_delete:
				continue
			
			idx_pred_arcs = succ_map.get(ni, [])
			idx_succ_arcs = pred_map.get(ni, [])
		
			if len(idx_pred_arcs) != 1 or len(idx_succ_arcs) != 1:
				continue
			
			idx_pred = idx_pred_arcs[0] 
			idx_succ = idx_succ_arcs[0]
		
			if not np.array_equal(allowed_dict[idx_pred], allowed_dict[idx_succ]):
				continue
			
			# Création du nouvel arc fusionné
			arc1 = arcs_records[idx_pred]
			arc2 = arcs_records[idx_succ]
		
			n1 = arc1['pred']
			l1 = arc1['lon']
			n2 = arc2['succ']
			l2 = arc2['lon']
		
			pmax = max(arc2['pmax'], arc1['pmax'])
		
			haut = arc2['haut']
			statut = arc2['statut']
			nbrob = arc2['nbrob']
			pressure = arc2['pressure']
		
			new_arc = {
				'pred': n1,
				'succ': n2,
				'lon': l1 + l2,
				'pmax': pmax,
				'haut': haut,
				'statut': statut,
				'nbrob': nbrob,
				'pressure': pressure
			}
		
			# Marquer les arcs et le noeud à supprimer, et ajouter le nouvel arc
			arcs_to_delete.add(idx_pred)
			arcs_to_delete.add(idx_succ)
			nodes_to_delete.add(ni)
		
			new_idx = new_arcs_counter
			new_arcs_counter += 1
		
			arcs_records[new_idx] = new_arc
			allowed_dict[new_idx] = allowed_dict[idx_pred]
		
			self.context['TableFusion'].append([ni, n1, l1, n2, l2, l1 + l2])
		
			# Mise à jour des listes d'adjacence
			succ_map[ni].remove(idx_pred)
			pred_map[ni].remove(idx_succ)
		
			succ_map[n2].remove(idx_succ)
			succ_map[n2].append(new_idx)
		
			pred_map[n1].remove(idx_pred)
			pred_map[n1].append(new_idx)

		final_arcs = [arcs_records[i] for i in arcs_records if i not in arcs_to_delete]
		final_allowed = [allowed_dict[i] for i in arcs_records if i not in arcs_to_delete]

		self.pipes = pd.DataFrame(final_arcs).astype(self.pipes.dtypes.to_dict())
		allowedPipes = np.array(final_allowed)
		self.nodes = TableNoeudsFusion[~TableNoeudsFusion['noeud'].isin(nodes_to_delete)].reset_index(drop=True)
		
		self.context['TableFusion'] = pd.DataFrame(self.context['TableFusion'], columns=[
			'noeud_supprime', 'extremite1', 'long1', 
			'extremite2', 'long2', 'lon_total'
		])

	def build_graph_artifcats(self):
		nbArcs = len(self.pipes)
		nbNodes = len(self.nodes)

		tn_idx = pd.Series(np.arange(nbNodes), index=self.nodes['noeud'])
		# Indices des prédécesseurs et successeurs
		predIdx = self.pipes['pred'].map(tn_idx).to_numpy()
		succIdx = self.pipes['succ'].map(tn_idx).to_numpy()

		row_idx = np.concatenate([predIdx, succIdx])
		col_idx = np.concatenate([np.arange(nbArcs), np.arange(nbArcs)])
		data = np.concatenate([-np.ones(nbArcs), np.ones(nbArcs)])

		Afull = sparse.coo_matrix((data, (row_idx, col_idx)), shape=(nbNodes, nbArcs)).tocsr()

		self.M_incid = Afull[succIdx, :]
		self.M_path = sparse.linalg.spsolve(self.M_incid.tocsc(), sparse.eye(nbArcs, format='csc'))
		self.idArcT = (self.pipes['statut'] == 2).to_numpy()
		self.Path2Term = self.M_path[:, self.idArcT].tocsc()

	def build_incidence_loads(self):
		"""
		BUILD_INCIDENCE_LOADS
		Computes estimated loads on arcs using the NW4 heuristic based on
		probabilistic demand aggregation.
		
		PRECONDITION
		------------
		dat.M_incid and dat.M_path must already exist and must have been
		constructed by build_graph_artifacts() after topology processing.
		
		Calcule TA.loadFactor à partir d'un arbre (pred->succ).
		- Entrées :
			TA(pred, succ, lon, ...)   (categorical pour pred/succ)
			TN(noeud, haut, nbrob, ...) (categorical pour noeud)
			pardsg : used to compute load factors
		- Output :
			TA (ajout du champ : loadFactor)
			TL (Table des charges (débits en litres/s))
			dat.idxPred, dat.idxSucc, dat.parentArc (indices utiles)
		"""
		if not (hasattr(self, 'M_incid') and hasattr(self, 'M_path')):
			raise RuntimeError("Graph matrices must be computed by build_graph_artifcats before calling build_incidence_loads.")
		
		nbArcs = len(self.pipes)
		# Grouping customers at each terminal
		G, borneU = pd.factorize(self.client['borne'], sort=True)
		nB = len(borneU)

		# ========= B Compute load factors ========

		# If the demand at the terminal nodes is to be satisfied, the 
		# stochastic demand induces a stochastic load on the arcs of the tree. 
		# When all users are the same, the distribution of flows on the path 
		# from the source to the terminal node follows a binomial law for each 
		# arc. This law is relevant to the design conditionally on the 
		# existence of at least one open faucet at the terminal node. When 
		# users have different attributes (demand, probability), explicit 
		# calculations are no longer possible. The distributions are evaluated 
		# using Monte Carlo sampling.    
		if self.param.get('uniform_dem') == 0:
			# --- 1- Simulate client's demand ----
			nS = self.param.get('nScenLoad', 20000)
			nC = len(self.client)

			rng = np.random.default_rng()
			scenarios = (rng.random((nC, nS)) < self.client['proba'].to_numpy()[:, None]) * self.client['cible'].to_numpy()[:, None]

			B2C = sparse.coo_matrix((np.ones(nC), (G, np.arange(nC))), shape=(nB, nC))
			# demande cumulée à chaque borne
			demande_borne = B2C @ scenarios
			
			isNodeFound = np.isin(borneU, self.nodes['noeud'])
			if not np.all(isNodeFound):
				raise ValueError("Impossible de faire correspondre toutes les bornes aux noeuds du graphe.")
			
			# --- - Flowload in the whole tree ----
			demande_noeud = np.zeros((nbArcs, nS))
			# Associe chaque borne (noeud terminal) à l'arc correspondant
			demande_noeud[self.idArcT, :] = demande_borne
			load_flow = self.M_path @ demande_noeud

			# --- - Define submatrices of path and flows  ----
			PathIdx = [self.Path2Term.indices[self.Path2Term.indptr[k]:self.Path2Term.indptr[k+1]] for k in range(nB)]
			ScenIdx = [np.where(demande_borne[k, :] > 0)[0] for k in range(nB)]

			# --- - Statistics on the nodal demands ----
			q_cond = np.zeros((nbArcs, nB))
			qos = self.param['servicequal']

			for k in range(nB):
				# Select scenarios with positive demand at terminal node k and
				# collect flows on the path from source to node k
				I = PathIdx[k] # arcs du chemin k
				J = ScenIdx[k] # scénarios actifs pour k
				
				if len(J) == 0:
					continue
					
				Xk = load_flow[I][:, J] # sous-matrice compacte
				r, m = Xk.shape
				# Compute percentile and the value just below
				# p/100*m est le nombre moyen d'éléments en prendre en compte.
				# Les percentiles sont associés au deux arrondis de ce nombre,
				# supérieur et inférieur.
				Y = np.sort(Xk, axis=1)
				idx1 = int(np.ceil(qos * m)) - 1 # L'arrondi est le numémax(Y,[],repmat(qc1,1,m),2);ro d'odre dans Y(k,:)
				qc1 = Y[:, idx1]
				
				lt = np.sum(Y < qc1[:, None], axis=1)
				le = np.sum(Y <= qc1[:, None], axis=1)
				pc2 = lt / m
				pc1 = le / m
				
				qc2 = np.zeros_like(qc1)
				for i in range(len(I)):
					if lt[i] == 0:
						qc2[i] = qc1[i]
					else:
						qc2[i] = Y[i, lt[i] - 1]
						
				# Combiner qc1 et qc2 en fonction en proportion des
				# probabilités et de l'objectif param.servicequal
				theta = np.ones_like(pc1)
				qc3 = np.copy(qc1)

				# Trouver les indices 
				idx3 = (pc1 > pc2) & (pc2 > 0)
				theta[idx3] = (qos - pc2[idx3]) / (pc1[idx3] - pc2[idx3])
				qc3[idx3] = theta[idx3] * qc1[idx3] + (1 - theta[idx3]) * qc2[idx3]

				# Reporter le résultat dans q_cond
				q_cond[I, k] = qc3
			
			self.loads = pd.DataFrame(q_cond, columns=borneU)

		elif self.param.get('uniform_dem') == 1:
			# Clients have identical attributes (NeatWork4)
			# --- Nombre de dépendants par arc ---
			nbdep = self.M_path @ self.pipes['nbrob'].to_numpy()
			self.pipes['nbdep'] = nbdep
			qos = self.param['servicequal']
			proba = self.param['opentaps']

			# sous-ensemble pertinent
			idx1 = nbdep > 1
			nbdep1 = nbdep[idx1] - 1

			# Encadrement sur le sous-ensemble
			k = binom.ppf(qos, nbdep1, proba)
			p2 = binom.cdf(k, nbdep1, proba)
			p1 = binom.cdf(k - 1, nbdep1, proba)

			# Approximation linéaire entre k et k-1. 
			# On part de k-1 et on ajoute l'écart entre qos et p1 relativement
			# à l'écart entre p2 et p1. Ces mesures se font pour un nombre de 
			# tirages nbdep - 1, qui est le nombre de robinets dépendants en
			# sus du robinet qu'on sait ouvert. La charge ld doit réintroduire
			# le robinet au'on sait ouvert. On aura k -1 + 1 = k pour la
			# charge.
			# Si k = 0, alors p1 = proba(X ≤ k-1<0) = 0; Donc p1>0 => k>=1.
			idx_aprox = (p2 > p1) & (p1 > 0)
			ld = np.ones_like(nbdep1, dtype=float)
			ld[idx_aprox] = k[idx_aprox] + (qos - p1[idx_aprox]) / (p2[idx_aprox] - p1[idx_aprox])
			
			idx_exact = (p2 == p1) & (p1 > 0) # indices du cas particulier
			ld[idx_exact] = k[idx_exact] # on prend directement k

			# Reporter sur les coordonnées pertinentes (idx1)
			loadFactor = np.ones(nbArcs, dtype=float)
			loadFactor[idx1] = ld
			self.pipes['loadFactor'] = loadFactor

			# Convert loadFactor in flows. Une borne de demande nulle n'est pas
			# supprimée automatiquement du réseau par le module de design. Elle 
			# reçoit un équipement minimal. Sa suppression éventuelle relève 
			# d'une décision explicite du concepteur.
			flow_eps = 1e-12
			flowFactor = np.maximum(loadFactor * self.param['targetflow'], flow_eps)

			# Report flowFactors on each path to terminal
			lf = self.Path2Term.multiply(flowFactor[:, None])
			self.loads = pd.DataFrame(lf.toarray(), columns=borneU)
			


# instance = DesignData("Toy", "_bbd", "dsg")
# instance.validate_and_align()
# res = instance.peel_orient_tree()
# instance.enrich_tables()
# # print(instance.classify_case())
# # instance.fusion_nodes_degre_2()
# instance.build_graph_artifcats()
# instance.build_incidence_loads()
# print(instance.loads)
