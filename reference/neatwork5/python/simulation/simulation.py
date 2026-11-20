from data import *
from utils import *
from solver import *
import numpy as np
import pandas as pd
import scipy.sparse as sparse
from tqdm import tqdm
import warnings
import matplotlib.pyplot as plt


def _safe_nanstats(data, mask, percentile_list=None):
	"""
	Safely compute min, mean, max with NaN handling for masked data.
	
	Parameters
	----------
	data : np.ndarray
		Input data array (1D or 2D).
	mask : np.ndarray
		Boolean mask to apply (True = keep, False = NaN).
	percentile_list : list, optional
		List of percentiles to compute (e.g., [5, 20]). If None, returns empty dict.
	
	Returns
	-------
	stats : dict
		Dictionary with keys 'min', 'mean', 'max' and optionally percentile keys.
	"""
	masked_data = np.where(mask, data, np.nan)
	
	with warnings.catch_warnings():
		warnings.simplefilter("ignore", category=RuntimeWarning)
		
		stats = {
			'min': np.nan_to_num(np.nanmin(masked_data, axis=1), nan=0.0),
			'mean': np.nan_to_num(np.nanmean(masked_data, axis=1), nan=0.0),
			'max': np.nan_to_num(np.nanmax(masked_data, axis=1), nan=0.0),
		}
		
		if percentile_list:
			for pct in percentile_list:
				pct_val = np.nanpercentile(masked_data, pct, axis=1, keepdims=True)
				pct_key = f'pct{pct}'
				cvar_key = f'cVar{pct}'
				stats[cvar_key] = np.nan_to_num(
					np.nanmean(np.where(masked_data <= pct_val, masked_data, np.nan), axis=1),
					nan=0.0
				)
	
	return stats


class Simulation:
	"""
	Class to run the flow simulations for multiple scenarios.
	Attributes
	----------
	inst : SimulationData
		The data instance containing the network and scenario information.
	demande_borne : np.ndarray
		Matrix of demand at each terminal for each scenario.
	nb_clients : np.ndarray
		Matrix of number of clients at each terminal for each scenario.
	sim_res : dict
		Dictionary to store the results of the simulations, including flows, satisfaction, iterations, etc.
	Methods
	-------
	__init__(self, inst)
		Initializes the simulation by generating scenarios and preparing result storage.
	genscen(self)
		Generates probabilistic scenarios of demand based on client probabilities.
	init_sim_res(self)
		Initializes the dictionary to store simulation results.
	prepare_sim(self, k)
		Prepares the data for the k-th scenario to be fed into the solver.
	runSimul(self)
		Runs the flow solver for all scenarios and stores the results.
	stat_processing(self)
		Processes the results of the simulations to compute statistics and prints them."""
	def __init__(self, inst: SimulationData, seed=None):
		self.inst = inst
		self.demande_borne, self.nb_clients = self.genscen(seed)
		self.sim_res = self.init_sim_res()
		
	def genscen(self, seed=None):
		"""
		Generate probabilistic scenarios of demand based on client probabilities.
		
		For each terminal, randomly draws open/closed status of clients based on 
		client probabilities, then sums their demands. Validates that at least
		one client is open per scenario.
		
		Returns
		-------
		demande_borne : np.ndarray (nBornes, nbsim)
			Aggregated demand at each terminal for each scenario (in m³).
		nb_clients : np.ndarray (nBornes, nbsim)
			Number of open clients at each terminal for each scenario.
		"""
		assert ((self.inst.clients['proba'] >= 0) & (self.inst.clients['proba'] <= 1)).all(), "Some probabilities are not in [0,1]"
		
		# ==== Mettre à jour TC.proba ====
		# Dans le cas NW4 de clients identiques il faut actualiser le choix de
		# la probbilité fait par l'utilisateur
		if self.inst.param['uniform_dem'] == 1:
			self.inst.clients['proba'] = self.inst.param['opentaps']
		
		# Tests pour se garantir contre le cas pathologique sans scénario
		assert (self.inst.clients['proba'] > 0).any(), "All probabilities are zero"
		
		# ==== Créer la marice des scénarios =====
		# Un scénario est défini par la liste des demandes cumulées en chaque	
		# borne. L'ensemble des scénarios est stocké sous forme de matrice.
		G, borneU = pd.factorize(self.inst.clients['borne'], sort=True) # borneU = matrice des scenarios
		nB = len(borneU)  # number of terminals
		nC = len(self.inst.clients)  # number of clients
		nS = self.inst.param['nbsim']  # number of scenarios
		
		# Aggregation matrix: maps clients to terminals
		ATC = sparse.coo_matrix((np.ones(nC), (G, np.arange(nC))), shape=(nB, nC))
		
		demande_borne = np.zeros((nB, nS))
		nb_clients = np.zeros((nB, nS))
		ouvert = np.zeros((nC, nS))
		
		nSc = 0
		iter = 0
		maxIter = 20
		
		rng = np.random.default_rng(seed)
		p_any = 1 - (1 - self.inst.clients['proba']).prod()
		
		# probabilité qu'il y ait au moins un client actif dans un scénario
		if p_any < 0.5:
			print("Very low expected number of valid scenarios in current draw.")
		# Si p_any >= 0.5, chaque itération donnera en moyenne la moitié de ce
		# qui est requis. Cela devrait suffire. Si le réseau est testé en	
		# période de charge p_any est proche de 1
		# Tirages aléatoires répétés pour garantir des scénarios avec au moins 
		# un client actif (demande > 0).
		while nSc < nS:
			iter += 1
			assert iter <= maxIter, "Scenario generation exceeded maximum iterations."
			
			nRemain = nS - nSc
			# nombre de colonnes à tirer pour obtenir ~nRemain scénarios utiles
			nDraw = int(np.ceil(1.1 * nRemain / p_any)) # facteur de sécurité 10%
			nDraw = max(nDraw, nRemain)                 # au minimum nRemain
			
			ouv = rng.random((nC, nDraw)) < self.inst.clients['proba'].to_numpy().reshape(-1, 1)
			keep = np.any(ouv, 0)
			ouv = ouv[:, keep]
			
			nbKeep = ouv.shape[1]
			nbTake = min(nbKeep, nRemain)
			
			if nbTake > 0:
				ouvert[:, nSc:nSc+nbTake] = ouv[:, :nbTake]
				nSc += nbTake
				
		scenarios = ouvert * self.inst.clients['cible'].to_numpy().reshape(-1, 1)  
		demande_borne = ATC @ scenarios
		nb_clients = ATC @ ouvert
		
		return demande_borne, nb_clients
	
	def init_sim_res(self):
		sim_res = {
			"X": np.zeros((self.inst.dat['nbA'], self.inst.param['nbsim'])),
			"Satis": np.zeros((self.inst.dat['nbAter'], self.inst.param['nbsim'])),
			"Ouv": np.zeros((self.inst.dat['nbA'], self.inst.param['nbsim'])),
			"Charge": np.zeros((self.inst.dat['nbA'], self.inst.param['nbsim'])),
			"Alpha": np.zeros((self.inst.dat['nbAter'], self.inst.param['nbsim'])),
			"Iter": np.zeros(self.inst.param['nbsim']),
			"U": np.zeros((self.inst.dat['nbNtra'], self.inst.param['nbsim'])),
			"dem_tot": np.zeros(self.inst.param['nbsim']),
		}
		return sim_res
	
	def prepare_sim(self, k: int):
		"""
		Prepare data for the k-th scenario to be fed into the solver.
		
		Configures alpha (resistance), bounds on flows, and constructs the
		system matrix for the bounded-flow optimization problem.
		
		Parameters
		----------
		k : int
			Scenario index (0 to nbsim-1).
		
		Returns
		-------
		work : dict
			Dictionary with keys: 'alpha', 'beta', 'xsup', 'xinf', 'closed', 'open',
			'A' (incidence matrix), 'ArcT' (terminal arcs), 'ArcB' (transit arcs),
			'ArcS' (source arcs), 'clients', 'gravit' (gravity).
		"""
		dem = self.demande_borne[:, k] * 1e-3 # Exprimer en m^3/s
		clients = np.zeros(len(self.inst.pipes))
		clients[self.inst.dat['idAter']] = self.nb_clients[:, k]
	
		# Indicateur des bornes actives dans l'ensemble des bornes
		ouvTer = dem > 0

		# indicateur des arcs porteurs de débit
		idactif = np.ones(len(self.inst.pipes), dtype=bool)
		idactif[self.inst.dat['idAter']] = ouvTer

		# indicateur des arcs terminaux (bornes) et  qui sont bornes actives.
		idTactif = idactif.copy()
		idTactif[~self.inst.dat['idAter']] = False

		# nombre de clients et demande cumulée par borne active
		nb_clients = self.nb_clients[ouvTer, k]
		dem_positive = dem[ouvTer]

		# ======== Calcul des alphas =========
		# Les coefficients de friction alpha sont nuls sauf pour les bornes	
		# actives.

		# --- alpha calculés pour les terminaux avec orifice ---
		if self.inst.param['uniform_dem'] == 1 and self.inst.param['bounded_flow'] == 0:
			# Cas NW4 avec un ou plusieurs robinets par terminal. Les alphas
			# ont été calculés à partir du diam des orifices et du
			# coefficient de friction des robinets param.rob. On s'assure
			# d'abord que ce calcul a bien été fait
			requis = ['alpha_com', 'alpha_ideal']
			if not all(col in self.inst.pipes.columns for col in requis):
				msg = ("Critical error: missing data.\n"
						"inst.pipes must have variables related to orifices and related alphas "    
						"when the design has uniform demand and bounded flow.")
				raise ValueError(msg)

			# Les alphas ont été calculés pour la perte de charge au robinet en
			# fonction du débit cible. S'il y a n robinets ouverts à un
			# terminal, le débit cible au terminal est n*cible. Le alpha
			# correspondant au terminal est le alpha aux robinets individuelss
			# divisé par n^2
			alpha = np.zeros(len(self.inst.pipes))
			if self.inst.param['orif_com'] == 1:
				alpha[idTactif] = self.inst.pipes.loc[idTactif, 'alpha_com'].to_numpy() / (clients[idTactif] ** 2)
			else:
				alpha[idTactif] = self.inst.pipes.loc[idTactif, 'alpha_ideal'].to_numpy() / (clients[idTactif] ** 2)
		elif self.inst.param['uniform_dem'] == 0 and self.inst.param['bounded_flow'] == 0:
			raise NotImplementedError("Ce cas n'est pas (encore) traité")
	
		else:
			# Il reste deux cas possibles. On a bounded_flow=1 et uniform_dem
			# vaut 0 ou 1. On est dans le cas irrigation.
			# du type irrigation ; et un seul cas si bounded_flow=0 qui est
			# traité comme NW4 avec plusieurs robinets.
			# Les clients ont un contrat de pression et de demande. Il n'y a
			# plus d'orifice par client, mais une contrôle de pression à la
			# borne.
			# Initialiser alpha
			alpha = np.zeros(len(self.inst.pipes))
			# Calculer les alphas aux bornes
			alpha[idTactif] = self.inst.param['pressure'] / (dem_positive ** 2)
			# alpha[idTactif] = ((self.inst.pipes.loc[idTactif, 'orif_ideal'].to_numpy()
			# 					/ 0.59)**(-4) + 1/self.inst.param['faucetcoef']) / nb_clients** 2
		# ==== Contraintes de boîtes sur les variables ===
		if self.inst.param['bounded_flow'] == 1:
			# sum(dem) est, en valeur absolue, une borne sur le débit pour tout arc
			xsup = np.sum(dem) * np.ones(self.inst.dat['nbA'])
			xinf = -xsup.copy()
			# Arcs terminaux
			xsup[self.inst.dat['idAter']] = dem
		else:
			xsup = 20 * np.sum(dem) * np.ones(self.inst.dat['nbA'])
			xinf = -xsup.copy()
			# Il n'y a pas de borne sur les débits aux bornes
			xsup[self.inst.dat['idAter']] = 20 * dem
		
		# borne inférieure aux terminaux et aux stations
		xinf[self.inst.dat['idAter']] = 0
		xinf[self.inst.dat['idAsrc']] = 0

		# --- Eliminer les arcs fermés ---
		closed_mask = (xinf == xsup)
		open_mask = ~closed_mask
		borne_active = self.nb_clients[:, k] > 0
	
		work = {
			'alpha': alpha[idactif],  # Valeur de alpha pour les variables actives dans débit
			'beta': self.inst.pipes.loc[idactif, 'beta'].to_numpy(), # beta récupérés des arcs actifs
			'xsup': xsup[open_mask],                      # bornes supérieures sur les débits
			'xinf': xinf[open_mask],                      # bornes inférieures sur les débits
			'closed': closed_mask,                        # masque des arcs fermés
			'open': open_mask,                            # masque des arcs ouverts
			'A': self.inst.dat['M_incid'][:, open_mask],  # matrice d'incidence réduite aux arcs ouverts
			'ArcT': self.inst.dat['idAter'][open_mask],   # indices des arcs terminaux ouverts
			'ArcB': self.inst.dat['idAtra'][open_mask],   # indices des arcs de transit ouverts
			'ArcS': self.inst.dat['idAsrc'][open_mask],   # indices des arcs sources ouverts
			'clients': self.nb_clients[borne_active, k],  # nombre de clients aux bornes actives
			'gravit': self.inst.dat['gravite'][open_mask] # gravité sur les arcs ouverts
		}
	
		return work

	def runSimul(self):
		ipm_solver = Solver(self.inst)
		nbsim = self.inst.param['nbsim']
		
		for k in tqdm(range(nbsim), desc="Résolution des scénarios"):
			try:
				work = self.prepare_sim(k)
				work = ipm_solver.debits(work)
				
				open_mask = work['open']
				
				self.sim_res["X"][open_mask, k] = work['x']
				self.sim_res["Iter"][k] = work['iter']
				self.sim_res["dem_tot"][k] = np.sum(self.demande_borne[:, k])
				
				self.sim_res["Ouv"][:, k] = open_mask
				self.sim_res["Charge"][open_mask, k] = work['charge']
				self.sim_res["U"][:, k] = work['u']
				
				dem_k = self.demande_borne[:, k] * 1e-3
				ouv_ter = dem_k > 0
				
				satis_k = np.zeros_like(dem_k)
				if self.inst.param['bounded_flow'] == 1:
					satis_k[ouv_ter] = work['x'][work['ArcT']] / work['xsup'][work['ArcT']]
				else:
					satis_k[ouv_ter] = work['x'][work['ArcT']] / work['clients'] / self.inst.param['targetflow'] / 1e-3
					
				self.sim_res["Satis"][:, k] = satis_k

			except RuntimeError:
				self.sim_res["Iter"][k] = -1
				
	def compute_stats(self):
		X = self.sim_res['X']
		Iter = self.sim_res['Iter']
		Ouv = self.sim_res['Ouv']
		OuvT = Ouv[self.inst.dat['idAter'], :].astype(bool)
		Charge = self.sim_res['Charge']
		U = self.sim_res['U']
		Satis = self.sim_res['Satis']

		sat_stats = _safe_nanstats(Satis, OuvT, percentile_list=[5, 20])
		
		borne = self.inst.nodes['noeud'][self.inst.dat['idNter']].to_numpy()
		TableSat = pd.DataFrame({
			'minSat': sat_stats['min'], 
			'moySat': sat_stats['mean'], 
			'maxSat': sat_stats['max'], 
			'cVarSat5': sat_stats['cVar5'], 
			'cVarSat20': sat_stats['cVar20']
		}, index=borne)

		stations = self.inst.nodes['noeud'][self.inst.dat['idNsrc']].to_numpy()
		Xst = X[self.inst.dat['idAsrc'], :]
		
		pct5_st = np.percentile(Xst, 5, axis=1, keepdims=True)
		cVarSt5 = np.nan_to_num(np.nanmean(np.where(Xst <= pct5_st, Xst, np.nan), axis=1), nan=0.0) * 1000
		
		pct95_st = np.percentile(Xst, 95, axis=1, keepdims=True)
		cVarSt95 = np.nan_to_num(np.nanmean(np.where(Xst >= pct95_st, Xst, np.nan), axis=1), nan=0.0) * 1000

		minSt = np.min(Xst, axis=1) * 1000
		moySt = np.mean(Xst, axis=1) * 1000
		maxSt = np.max(Xst, axis=1) * 1000
		TableSt = pd.DataFrame({
			'minSt': minSt, 'moySt': moySt, 'maxSt': maxSt, 
			'cVarSt5': cVarSt5, 'cVarSt95': cVarSt95
		}, index=stations)

		idAinter = self.inst.dat['idAtra'] | self.inst.dat['idAsrc']
		xb = np.abs(X[idAinter, :])
		diam1 = self.inst.pipes['diam1'].to_numpy()
		surface = np.pi * diam1[idAinter]**2 / 4.0
		Vit = xb / surface[:, None]
		
		minV = np.min(Vit, axis=1)
		moyV = np.mean(Vit, axis=1)
		maxV = np.max(Vit, axis=1)
		
		fin = self.inst.pipes['succ'][idAinter].to_numpy()
		debut = self.inst.pipes['pred'][idAinter].to_numpy()
		TableResV = pd.DataFrame({'debut': debut, 'fin': fin, 'minV': minV, 'moyV': moyV, 'maxV': maxV})

		chargeT = Charge[self.inst.dat['idAter'], :]
		
		press_stats = _safe_nanstats(-chargeT, OuvT, percentile_list=None)
		
		TablePress = pd.DataFrame({
			'minP': press_stats['min'], 
			'moyP': press_stats['mean'], 
			'maxP': press_stats['max']
		}, index=borne)

		itesim = self.inst.param['nbsim']
		tnB = self.inst.nodes['noeud'][self.inst.dat['idNtra']].to_numpy()
		haut = self.inst.nodes['haut'][self.inst.dat['idNtra']].to_numpy()
		prB = -np.tile(haut[:, None], (1, itesim)) + U
		minPrB = np.min(prB, axis=1)
		moyPrB = np.mean(prB, axis=1)
		maxPrB = np.max(prB, axis=1)
		TablePrB = pd.DataFrame({'minPrB': minPrB, 'moyPrB': moyPrB, 'maxPrB': maxPrB}, index=tnB)

		self.stats = {
			'TableSat': TableSat,
			'TableSt': TableSt,
			'TableResV': TableResV,
			'TablePress': TablePress,
			'TablePrB': TablePrB,
			'Iter': Iter,
			'borne': borne,
		}

	def display_stats(self, print_flag=True):
		Satborne = '\n Satisfaction aux bornes'
		xst = '\n Débits aux stations'
		vitesse = '\n Vitesses dans les arcs intermédiaires en m/s'
		pborne = '\n Pressions en amont des orifices en mètres'
		pressB = '\n Excès de pression aux nœuds intermédiaires pendant les écoulements'
		
		Iter = self.stats['Iter']
		resIter = f"\nIterations : min = {np.min(Iter)}, moyenne = {np.mean(Iter):.2f}, max = {np.max(Iter)}"

		if print_flag:
			print(Satborne)
			print(self.stats['TableSat'])
			print(xst)
			print(self.stats['TableSt'])
			print(vitesse)
			print(self.stats['TableResV'])
			print(pborne)
			print(self.stats['TablePress'])
			print(pressB)
			print(self.stats['TablePrB'])
			print(resIter)

			if 'TempsIPM' in self.sim_res:
				print(f"Temps de calcul des débits {self.sim_res['TempsIPM']}")

	def export_stats(self, export_flag=True):
		if export_flag:
			filename = f"simul_debit_{self.inst.name}_{self.inst.version}.xlsx"
			with pd.ExcelWriter(filename, engine='openpyxl') as writer:
				self.stats['TableSat'].to_excel(writer, sheet_name='Sheet1', startrow=1, startcol=1, index_label='Bornes')
				self.stats['TableSt'].to_excel(writer, sheet_name='Sheet2', startrow=1, startcol=1)
				self.stats['TableResV'].to_excel(writer, sheet_name='Sheet3', startrow=1, startcol=1, index=False)
				self.stats['TablePress'].to_excel(writer, sheet_name='Sheet4', startrow=1, startcol=1)
				self.stats['TablePrB'].to_excel(writer, sheet_name='Sheet5', startrow=1, startcol=1)
		print("Fin du traitement statistique.")

	def stat_processing(self, print_flag=True, export_flag=False):
		self.compute_stats()
		self.display_stats(print_flag=print_flag)
		self.export_stats(export_flag=export_flag)

	def compare_stats(self, filename=None):
		if filename is None:
			filename = f"../test_examples/simul_debit_{self.inst.name}_{self.inst.version}.xlsx"
		
		sheets = {
			'Sheet1': ('TableSat', True),
			'Sheet2': ('TableSt', True),
			'Sheet3': ('TableResV', False),
			'Sheet4': ('TablePress', True),
			'Sheet5': ('TablePrB', True)
		}
		
		for sheet, (name, has_index) in sheets.items():
			df_calc = self.stats[name]
			idx_col = 1 if has_index else None
			
			df_excel = pd.read_excel(filename, sheet_name=sheet, skiprows=1, index_col=idx_col)
			df_excel = df_excel.dropna(axis=1, how='all')
			
			num_calc = df_calc.select_dtypes(include=[np.number]).to_numpy()
			num_excel = df_excel.select_dtypes(include=[np.number]).to_numpy()
			
			if num_calc.shape == num_excel.shape:
				max_diff = np.nanmax(np.abs(num_calc - num_excel))
				if max_diff > 1e-6:
					print(f"{name} : {max_diff:.4e}")
				else:
					print(f"{name} : OK")
			else:
				print(f"{name} : {num_calc.shape} vs {num_excel.shape}")