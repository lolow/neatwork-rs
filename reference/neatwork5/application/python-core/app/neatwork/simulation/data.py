import os
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import sparse

from .constants import DEFAULT_SIMULATION_PARAMS
from .numerical_utils import get_section_indices, map_diam_cost

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
        """
        Initialize the data class with network identification and parse the .dsg file.
        
        Parameters
        ----------
        name : str
            Network name identifier
        version : str
            Data format version
        output : str
            Output format specification
        dossier : str
            Directory path containing the .dsg file
        file_name : str
            Name of the .dsg file (without extension)
        """
        # Store basic network identification parameters
        self.name = name
        self.version = version
        self.output = output
        self.dossier = dossier
        self.file_name = file_name

        # Initialize solver options with default values
        self.options = self.init_options()
        self.is_prepared = False
        self.dat: dict = {}
        
        # Parse the .dsg file into pandas DataFrames and parameter dictionary
        # This is the main data loading step that populates all data attributes
        self.nodes, self.pipes, self.clients, self.bornes, self.diameters, self.param = self.parse_dsg_to_df()

        self.validate()
        
    

    @classmethod
    def from_tables(cls, *, name: str, nodes: pd.DataFrame, pipes: pd.DataFrame, clients: pd.DataFrame, bornes: pd.DataFrame, diameters: pd.DataFrame, params: dict[str, float | int], version: str = "", source_format:str = "DSG",) -> "SimulationData":
        """
        Create a SimulationData instance directly from pre-loaded tables.
        
        This class method allows bypassing the .dsg file parsing step by providing
        the required data tables and parameters directly.
        
        """
        instance = cls.__new__(cls)
        instance.name = name
        instance.version = version
        instance.output = source_format.lower()
        instance.dossier = None
        instance.file_name = None

        instance.options = instance.init_options()
        instance.is_prepared = False
        instance.dat = {}

        instance.nodes = nodes.copy(deep=True)
        instance.pipes = pipes.copy(deep=True)
        instance.clients = clients.copy(deep=True)
        instance.bornes = bornes.copy(deep=True)
        instance.diameters = diameters.copy(deep=True)
        
        instance.param = {
            **DEFAULT_SIMULATION_PARAMS,
            **params,
        }

        instance.validate()
        return instance
    
    def validate(self) -> None:
        """Validate raw simulation data before preprocessing."""
        from .validation import validate_simulation_data

        validate_simulation_data(self)

    def __str__(self):
        return (f"Nom de l'instance: {self.name}\n"
                f"Version: {self.version}\n"
                f"Type de sortie: {self.output}\n")
    
    def get_path(self, sub_dir):
        ROOT_DIR = Path(__file__).resolve().parent.parent.parent
        SUB_DIR = ROOT_DIR / sub_dir / self.name
        return SUB_DIR
    
    def parse_dsg_to_df(self):
        """
        Parse the .dsg file and return the data as pandas DataFrames.
        The .dsg file format contains sections delimited by keywords:
        - Nodes: Node coordinates and properties
        - Pipes: Arc connections between nodes with lengths and diameters
        - Clients: Demand nodes with consumption probabilities
        - Diameters: Available pipe diameters with costs and properties
        - Default properties: Simulation parameters
        - Borne: Terminal node pressure specifications (optional)
        
        Returns
        -------
        tuple
            (df_nodes, df_pipes, df_clients, df_bornes, df_diameters, param)
            Six objects containing all parsed network data
            
        Raises
        ------
        FileNotFoundError
            If the .dsg file does not exist
        ValueError
            If any required section is missing from the file
        """
        # Build the full path to the .dsg file
        dsg_path = self.get_path(self.dossier)
        dsg_file = os.path.join(dsg_path, f"{self.file_name}.dsg")
        
        # Verify that the file exists before attempting to read it
        if not os.path.exists(dsg_file):
            raise FileNotFoundError(f"Le fichier {dsg_file} n'existe pas.")
        
        # Read the file and strip whitespace from each line
        with open(dsg_file, 'r') as file:
            raw_lines = [line.strip() for line in file.readlines()]
            # Process the raw lines to extract the data
            # Find the start and end indices of each section in the file
            # These indices are used to extract the relevant data blocks
            idx_client = get_section_indices(raw_lines, "Clients")
            idx_nodes = get_section_indices(raw_lines, "Nodes")
            idx_pipes = get_section_indices(raw_lines, "Pipes")
            idx_prop = get_section_indices(raw_lines, "Default Properties")
            idx_diameters = get_section_indices(raw_lines, "Diameters")
            idx_bornes = get_section_indices(raw_lines, "Bornes")

            # Check if any of the required sections are missing for simulation
            missing = []
            if idx_client is None: missing.append("Clients")
            if idx_nodes is None: missing.append("Nodes")
            if idx_pipes is None: missing.append("Pipes")
            if idx_prop is None: missing.append("Default Properties")
            if idx_diameters is None: missing.append("Diameters")

            # Raise an error if any required section is missing
            if missing:
                raise ValueError(f"Les sections suivantes sont manquantes dans le fichier .dsg: {', '.join(missing)}")
            
            lines = [[val.strip() for val in line.split(',')] for line in raw_lines]

            # ================= NODES =================
            # Extract node data: vertices of the graph and their properties
            # Nodes can be sources (statut=0), transit (statut=1), or terminals (statut=2)
            cols = ['noeud', 'haut', 'xdata', 'ydata', 'orif_ideal', 'orif_com', 'statut']
            df_nodes = pd.DataFrame(lines[idx_nodes[0]:idx_nodes[1]], columns=cols)
            # Convert numeric columns to proper numeric types
            df_nodes[['haut', 'xdata', 'ydata', 'orif_ideal', 'orif_com', 'statut']] \
                = df_nodes[['haut', 'xdata', 'ydata', 'orif_ideal', 'orif_com', 'statut']].apply(pd.to_numeric)
            # Ensure node IDs are strings for consistent indexing
            df_nodes['noeud'] = df_nodes['noeud'].astype(str)
            # Sort nodes by status then ID for consistent ordering
            df_nodes = df_nodes.sort_values(by=['statut', 'noeud']).reset_index(drop=True)

            # ================== ARCS (TA) ==================
            # Extract pipe/arc data: graph edges with their properties
            # Arcs are oriented from predecessor (pred) to successor (succ)
            cols = ['pred','succ','lon', 'lon1', 'ID1', 'lon2', 'ID2', 'N']
            df_pipes = pd.DataFrame(lines[idx_pipes[0]:idx_pipes[1]], columns=cols)
            # Convert numeric columns to proper numeric types
            df_pipes[['lon','lon1','lon2']] = df_pipes[['lon','lon1','lon2']].apply(pd.to_numeric)
            # Ensure node IDs are strings for consistent indexing
            df_pipes['pred'] = df_pipes['pred'].astype(str)
            df_pipes['succ'] = df_pipes['succ'].astype(str)
            # Add an external edge ID for each pipe for UI mapping and result association
            df_pipes.insert(0, "external_edge_id", [f"A{index + 1}" for index in range(len(df_pipes))])
            # Sort pipes by successor then predecessor for consistent ordering
            df_pipes = df_pipes.sort_values(by=['succ', 'pred']).reset_index(drop=True)

            # ================== DIAMETRES (TD) ==================
            # Extract available pipe diameters with their properties and costs
            # This table maps diameter IDs to physical diameters, costs, and hydraulic properties
            cols = ['ID','Nominal','SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness', 'N']
            df_diameters = pd.DataFrame(lines[idx_diameters[0]:idx_diameters[1]], columns=cols)
            # Convert numeric columns to proper numeric types
            df_diameters[['SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness']] = \
                df_diameters[['SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness']].apply(pd.to_numeric)
            # Ensure diameter IDs are strings for consistent indexing
            df_diameters['ID'] = df_diameters['ID'].astype(str)

            # ================== PARAMETRES (TP) ==================
            # Extract simulation parameters with default fallback values
            # These parameters control the hydraulic simulation behavior
            
            # Define default parameters that will be used if not specified in the file
            
            # Parse parameters from the file (override defaults if present)
            params = {line[0]: float(line[1]) for line in lines[idx_prop[0]:idx_prop[1]]}
            # Merge defaults with file parameters (file values take precedence)
            param = {**DEFAULT_SIMULATION_PARAMS, **params}

            # ================== CLIENTS (TC) ==================
            # Extract client data: demand nodes with consumption probability and target
            cols = ['client','borne','proba','cible']
            df_clients = pd.DataFrame(lines[idx_client[0]:idx_client[1]], columns=cols)
            # Convert probability and target to numeric types
            df_clients[['proba','cible']] = df_clients[['proba','cible']].apply(pd.to_numeric)
            # Ensure client IDs are strings for consistent indexing
            df_clients['client'] = df_clients['client'].astype(str)
            df_clients['borne'] = df_clients['borne'].astype(str)
            
            # Check if all clients have the same probability and target
            # If not, use non-uniform demand mode with bounded flow
            if df_clients['proba'].nunique() != 1 or df_clients['cible'].nunique() != 1:
                param['uniform_dem'] = 0 # Non-uniform demand pattern
                param['bounded_flow'] = 1 # Enable flow bounds
            else:
                param['uniform_dem'] = 1 # Uniform demand pattern

            # ================== BORNES (TB) ==================
            # Extract terminal node pressure specifications
            # Terminals are nodes with statut=2 (consumption nodes)
            cols = ['borne','pressure', 'N']
            maskTerm = df_nodes['statut'] == 2
            terminal = df_nodes.loc[maskTerm, 'noeud'].astype(str)
            
            # If Borne section exists, use it; otherwise create default pressure values
            if idx_bornes is not None:
                df_bornes = pd.DataFrame(lines[idx_bornes[0]:idx_bornes[1]], columns=cols)
                df_bornes['pressure'] = pd.to_numeric(df_bornes['pressure'])
                df_bornes['borne'] = df_bornes['borne'].astype(str)
                df_bornes = df_bornes.sort_values(by='borne').reset_index(drop=True)
                
                # Validate that Borne section matches terminal nodes
                if set(df_bornes['borne'].unique()) != set(terminal):
                    raise ValueError("Les bornes ne correspondent pas aux noeuds terminaux")
            else:
                # Create default borne data with uniform pressure from parameters
                df_bornes = pd.DataFrame({
                    'borne': terminal,
                    'pressure': param['pressure']
                }).sort_values(by='borne').reset_index(drop=True)

        # Return all extracted and processed data
        return df_nodes, df_pipes, df_clients, df_bornes, df_diameters, param


    def completeTables(self):
        """
        Backward-compatible wrapper for the former preprocessing method.

        New code should call prepare().
        """
        self.prepare()
    
    def complete_data(self):
        """
        Backward-compatible wrapper for the former solver-data method.

        New code should call prepare().
        """
        if not self.is_prepared:
            self.prepare()

    def prepare(self) -> "SimulationData":
        """
        Validate and preprocess the network for hydraulic simulation.

        Returns
        -------
        SimulationData
            The same instance, enriched with derived pipe columns and
            solver-ready data.
        """

        from .preprocessor import HydraulicPreprocessor

        self.validate()

        prepared = HydraulicPreprocessor(self).prepare()

        self.nodes = prepared.nodes
        self.pipes = prepared.pipes
        self.clients = prepared.clients
        self.bornes = prepared.bornes
        self.diameters = prepared.diameters
        self.dat = prepared.solver_data
        self.is_prepared = True

        return self

    def init_options(self):
        """
        Initialize options for the IPM (Interior Point Method) solver.
        
        Returns
        -------
        dict
            Solver configuration options with default values
        """
        return {"tolx": 1e-8,       # Flow tolerance (1 milliliter per second)
                "tolpress": 1e-6,	# Pressure tolerance (1mm of water ≈ 1e-4 bar)
                "tolpseudoA": 1e-10,# Tolerance for declaring pseudo-active constraints
                "maxiter": 30,      # Maximum iterations for Newton's method
                "tolcent": 1e-10,   # Centrality tolerance parameter for y regularization
                "sigma": 0.1,       # reduction factor for mu (barrier parameter)
                "tau": 2.5,         # exponent used in sigma_mu = min(1,(mu_aff/mu)^tau)*mu
                "a_secur": 0.999}   # safeguard on final step size
    
    def incidence_intermediaire(self):
        """
        Backward-compatible access to the prepared incidence matrix.
        """
        if not self.is_prepared:
            self.prepare()

        return self.dat["M_incid"]

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
            



