import numpy as np

def get_section_indices(lines:list[str], section_name:str):
	start, end = -1, -1
	for i, line in enumerate(lines):
		if line.startswith(f"!{section_name}"):
			start = i + 1
			continue 
		if start != -1:
			if line.startswith('!'):
				end = i
				break
			elif i == len(lines) - 1:
				end = i + 1
				break
	if start==-1 or end==-1:return None
	return start, end

def degree_counts_simple(pred_idx, succ_idx, activeArcs, activeNodes, n):
	"""
	Calcule le degré de chaque noeud en comptabilisant uniquement les connexions actives
	"""
	active_p = pred_idx[activeArcs]
	active_s = succ_idx[activeArcs]
	
	valid_p = active_p[activeNodes[active_p]]
	valid_s = active_s[activeNodes[active_s]]
	
	degCounts = np.bincount(valid_p, minlength=n) + np.bincount(valid_s, minlength=n)
	return degCounts

def buildAllowedPipes(edges, diameters):
	"""
		buildAllowedPipes : construit la matrice (arc × diamètre) des diamètres autorisés
	
		Inputs:
			edges : table des arcs, doit contenir la colonne 'pmax'
			diameters : table des diamètres, doit contenir la colonne 'pressure'
	
		Output:
			AllowedPipes : matrice logique (nombre d'arcs × nomnre de diamètres)
	"""
	if 'pmax' not in edges.columns:
		raise ValueError("edges doit contenir la colonne pmax")

	if 'pressure' not in diameters.columns:
		raise ValueError("diameters doit contenir la colonne pressure")

	AllowedPipes = np.less_equal.outer(edges['pmax'].to_numpy(), diameters['pressure'].to_numpy())
	return AllowedPipes