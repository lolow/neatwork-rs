import java.awt.Point;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.io.InputStream;
import java.util.*;

import neatwork.Messages;
import neatwork.file.FileManagerDisk;
import neatwork.project.*;
import neatwork.utils.MapComparator;
import neatwork.utils.Tools;

/** Code copied verbatim from the NeatWork GUI classes, to record expected outputs. */
public class Oracle {
	static Path out;
	static Path samples;

	public static void main(String[] args) throws Exception {
		Locale.setDefault(Locale.ENGLISH);
		Messages.setLocale(Locale.ENGLISH);
		samples = Path.of(args[0]);
		out = Path.of(args[1]);
		Files.createDirectories(out);
		Properties app = loaderProperties(args[2]);

		Topographie el = new Topographie("El_Bihajua", read("El_Bihajua.tpo"), app);
		Topographie sample = new Topographie("SampleNetwork", read("SampleNetwork.tpo"), app);
		Topographie base = new Topographie("baseexample", read("baseexample.tpo"), app);
		Design toy = new Design("toy", read("toy.dsg"), app);
		Design eld = new Design("El_Bihajua", read("El_Bihajua.dsg"), app);

		// 1. tree
		write("tree_El_Bihajua_tpo.txt", tree(el));
		write("tree_SampleNetwork_tpo.txt", tree(sample));
		write("tree_baseexample_tpo.txt", tree(base));
		write("tree_toy_dsg.txt", tree(toy));
		write("tree_El_Bihajua_dsg.txt", tree(eld));

		// 2. stats
		write("stats_El_Bihajua_tpo.html", topoStat(el));
		write("stats_SampleNetwork_tpo.html", topoStat(sample));

		// 3. quick check
		write("quickcheck_El_Bihajua_tpo.txt", quickCheck(el));
		write("quickcheck_SampleNetwork_tpo.txt", quickCheck(sample));

		// 4. cost
		write("cost.txt", costLabel(toy) + "\n" + toy.getCost() + "\n" + costLabel(eld) + "\n" + eld.getCost() + "\n");

		// 6. reports (before extract: extract changes nbTaps)
		write("report_El_Bihajua_tpo.html", reportTopo(el));
		write("report_toy_dsg.html", reportDesign(toy));
		write("report_El_Bihajua_dsg.html", reportDesign(eld));
		Simulation s0 = new Simulation("toy.0");
		s0.setContent(read("toy.0.sim"));
		write("report_toy_0_sim.html", reportSimu(s0, toy));
		Simulation s1 = new Simulation("El_Bihajua.0");
		s1.setContent(read("El_Bihajua.0.sim"));
		write("report_El_Bihajua_0_sim.html", reportSimu(s1, eld));
		Simulation s2 = new Simulation("toy.1");
		s2.setContent(read("toy.1.sim"));
		write("report_toy_1_sim.html", reportSimu(s2, toy));
		for (String n : new String[] { "toy.3", "toy.6" }) {
			Simulation sx = new Simulation(n);
			sx.setContent(read(n + ".sim"));
			write("report_" + n.replace('.', '_') + "_sim.html", reportSimu(sx, toy));
		}
		Database db = new Database(new FileManagerDisk(app), app);
		write("database_diameters.html", dbDiameters(db));
		write("database_orifices.html", dbOrifices(db));

		// 5. extract
		write("extract_toy_dsg.tpo", toy.extractTopoContent());
		write("extract_El_Bihajua_dsg.tpo", eld.extractTopoContent());

		// 7. naming
		StringBuilder sb = new StringBuilder();
		String[][] cases = { { "toy" }, { "toy", "toy" }, { "toy", "toy", "toy (2)" }, { "toy (2)", "toy (2)" },
				{ "toy (2)", "toy (3)" }, { "toy (x)", "toy (x)" }, { "a(5)", "a(5)", "a (5)" }, { "toy)(", "toy)(" } };
		for (String[] c : cases) {
			Vector<String> list = new Vector<>();
			for (int i = 1; i < c.length; i++)
				list.add(c[i]);
			sb.append(c[0]).append(" -> ").append(isAlreadyLoaded(c[0], list)).append("\n");
		}
		for (String[] c : cases) {
			String[] f = Arrays.copyOfRange(c, 1, c.length);
			sb.append(c[0]).append(" => ").append(getFirstNameCompatible(c[0], f)).append("\n");
		}
		write("naming.txt", sb.toString());
	}

	static Properties loaderProperties(String path) throws Exception {
		Properties p = new Properties();
		try (InputStream in = Oracle.class.getResourceAsStream("/neatwork/default.properties")) {
			p.load(in);
		}
		p.setProperty("appli.standalone", "true");
		p.setProperty("solver.distant", "false");
		p.setProperty("file.distant", "false");
		p.setProperty("file.path", path);
		Properties po = new Properties();
		po.setProperty("appli.locale", "en");
		p.putAll(po);
		return p;
	}

	static String read(String f) throws Exception {
		return Files.readString(samples.resolve(f), StandardCharsets.UTF_8);
	}

	static void write(String f, String s) throws Exception {
		Files.writeString(out.resolve(f), s, StandardCharsets.UTF_8);
	}

	// ------------------------------------------------------------ tree
	static String tree(Network network) {
		if (!network.isATree()) {
			return "_" + network.getName() + Messages.getString("TreeNetwork._is_not_a_tree") + "\n";
		}
		TreeOracle t = new TreeOracle(network);
		Hashtable coord = t.buildTree();
		int nbrow = 0, nbcol = 0;
		Iterator iter = coord.keySet().iterator();
		StringBuilder order = new StringBuilder();
		while (iter.hasNext()) {
			Point item = (Point) iter.next();
			nbcol = Math.max(nbcol, item.x + 1);
			nbrow = Math.max(nbrow, item.y + 1);
			order.append(item.y).append(",").append(item.x).append(",").append(coord.get(item)).append("\n");
		}
		StringBuilder sb = new StringBuilder();
		sb.append("rows ").append(nbrow).append(" cols ").append(nbcol).append("\n");
		for (int r = 0; r < nbrow; r++) {
			for (int c = 0; c < nbcol; c++) {
				Object o = coord.get(new Point(c, r));
				sb.append(c == 0 ? "" : "|").append(o == null ? "" : o.toString());
			}
			sb.append("\n");
		}
		sb.append("order\n").append(order);
		return sb.toString();
	}

	static class TreeOracle {
		private Network network;
		private Hashtable nodePath = new Hashtable();

		TreeOracle(Network n) {
			network = n;
		}

		// copied from TreeNetwork.buildTree
		public Hashtable buildTree() {
			nodePath.clear();
			Map map = new Hashtable();
			Iterator iter = network.getNodeIterator();
			while (iter.hasNext()) {
				Node item = (Node) iter.next();
				map.put(item.getName(), new Integer(0));
				nodePath.put(item.getName(), new Integer(0));
			}
			profondeur(network.getSource(), map, 0);
			int prof = 0;
			Vector nprof = new Vector();
			while (!map.isEmpty()) {
				Vector line = new Vector();
				iter = map.keySet().iterator();
				while (iter.hasNext()) {
					String n = iter.next().toString();
					Integer p = (Integer) map.get(n);
					if (p.intValue() == prof) {
						line.add(n);
						iter.remove();
					}
				}
				nprof.add(line);
				prof++;
			}
			map = new Hashtable();
			iter = network.getNodeIterator();
			while (iter.hasNext()) {
				Node item = (Node) iter.next();
				map.put(item.getName(), new Integer(0));
			}
			iter = network.getNodeIterator();
			while (iter.hasNext()) {
				Node item = (Node) iter.next();
				if (item.getType() == Node.TYPE_FAUCET) {
					remonteSource(item, map);
				}
			}
			Enumeration e = nprof.elements();
			Comparator comparator = new MapComparator(map);
			while (e.hasMoreElements()) {
				Vector item = (Vector) e.nextElement();
				Collections.sort(item, comparator);
			}
			Map mapX = new Hashtable();
			for (int i = 0; i < nprof.size(); i++) {
				Vector line = (Vector) nprof.get(i);
				for (int j = 0; j < line.size(); j++) {
					mapX.put(line.get(j), new Integer(i));
				}
			}
			Map mapY = new Hashtable();
			mapY.put(network.getSource().getName(), new Integer(0));
			for (int i = 0; i < (nprof.size() - 1); i++) {
				Vector line = (Vector) nprof.get(i);
				Vector lineSuiv = (Vector) nprof.get(i + 1);
				for (int j = 0; j < line.size(); j++) {
					String node = line.get(j).toString();
					int pos = ((Integer) mapY.get(node)).intValue();
					for (int k = 0; k < lineSuiv.size(); k++) {
						String succ = lineSuiv.get(k).toString();
						if (network.isSucc(node, succ)) {
							mapY.put(succ, new Integer(pos));
							pos += (((Integer) map.get(succ)).intValue());
						}
					}
				}
			}
			Hashtable coord = new Hashtable();
			iter = mapX.keySet().iterator();
			while (iter.hasNext()) {
				String item = iter.next().toString();
				coord.put(new Point(((Integer) mapX.get(item)).intValue(), ((Integer) mapY.get(item)).intValue()), item);
			}
			for (int i = 0; i < 2; i++) {
				iter = network.getNodeIterator();
				while (iter.hasNext()) {
					Node item = (Node) iter.next();
					int y = ((Integer) mapY.get(item.getName())).intValue();
					int x = ((Integer) mapX.get(item.getName())).intValue();
					if ((y > 0) && (coord.get(new Point(x - 1, y)) == null)) {
						coord.put(new Point(x - 1, y), "_*L");
						x--;
						y--;
						Object z = coord.get(new Point(x, y));
						while ((y > 0) && (z == null)) {
							coord.put(new Point(x, y), "_*I");
							y--;
							z = coord.get(new Point(x, y));
						}
						if ((z.toString().equals("_*I")) || (z.toString().equals("_*L"))) {
							coord.put(new Point(x, y), "_*T");
						}
					} else if ((y > 0) && (coord.get(new Point(x - 1, y)).toString().equals("_*I"))) {
						coord.put(new Point(x - 1, y), "_*T");
					}
				}
			}
			return coord;
		}

		public void profondeur(Node node, Map map, int prof) {
			map.put(node.getName(), new Integer(prof));
			Iterator iter = network.getPipeIterator();
			while (iter.hasNext()) {
				Pipe p = (Pipe) iter.next();
				if (p.getBegin().equals(node.getName())) {
					profondeur(network.getNode(p.getEnd()), map, prof + 1);
				}
			}
		}

		public void remonteSource(Node node, Map map) {
			Integer i = (Integer) map.get(node.getName());
			i = new Integer(i.intValue() + 1);
			map.put(node.getName(), i);
			if (node.getType() != Node.TYPE_RESERVOIR) {
				remonteSource(network.getFirstPred(node), map);
			}
		}
	}

	// ------------------------------------------------------------ TopoStatDialog
	static String topoStat(Topographie topo) {
		return Messages.getString("TopoStatDialog.<HTML>_<blockquote><B>Topography_name__</B>") + topo.getName()
				+ " <BR><BR>" + Messages.getString("TopoStatDialog.<B>Number_of_Nodes__</B>") + topo.getNbNodes()
				+ "<UL>" + Messages.getString("TopoStatDialog.<LI>_<i>Branching_nodes__</i>")
				+ topo.getNbNodes(Node.TYPE_DISPATCH) + Messages.getString("TopoStatDialog.<LI>_<i>Faucet_nodes__</i>")
				+ topo.getNbNodes(Node.TYPE_FAUCET) + Messages.getString("TopoStatDialog._(with")
				+ topo.getNbTotalTaps() + Messages.getString("TopoStatDialog._individual_faucets)") + "</UL>"
				+ Messages.getString("TopoStatDialog.<B>Total_height_change__</B>")
				+ Tools.doubleFormat("#", topo.getTotalHeightChange()) + " m<BR><BR>"
				+ Messages.getString("TopoStatDialog.<B>Number_of_Pipes__</B>") + topo.getNbPipes() + " <BR>"
				+ Messages.getString("TopoStatDialog.<B>Total_length__</B>")
				+ Tools.doubleFormat("#", topo.getTotalLength()) + " m <BR>";
	}

	// ------------------------------------------------------------ QuickCheckTableModel
	static String quickCheck(Topographie topo) {
		double targetflow = Double.parseDouble(topo.getProperties().getProperty("topo.targetflow.value"));
		double seuil = Double.parseDouble(topo.getProperties().getProperty("topo.seuil.value", "0.1"));
		double alpha = Double.parseDouble(topo.getProperties().getProperty("topo.faucetcoef.value"));
		String[] myHeader = { Messages.getString("QuickCheckTableModel.Faucet_ID"),
				Messages.getString("QuickCheckTableModel.flow_min_(") + seuil
						+ Messages.getString("QuickCheckTableModel._l/s)"),
				Messages.getString("QuickCheckTableModel.target_flow_(") + targetflow
						+ Messages.getString("QuickCheckTableModel._l/s)") };
		StringBuilder sb = new StringBuilder();
		sb.append(String.join("\t", myHeader)).append("\n");
		Iterator iter = topo.getNodeIterator();
		while (iter.hasNext()) {
			Node node = (Node) iter.next();
			if (node.getType() == Node.TYPE_FAUCET) {
				sb.append(node.getName()).append("\t");
				sb.append((-node.getHeight() > (Math.pow(seuil / 1000, 2) / alpha))
						? Messages.getString("QuickCheckTableModel.Ok")
						: Messages.getString("QuickCheckTableModel.No"));
				sb.append("\t");
				sb.append((-node.getHeight() > (Math.pow(targetflow / 1000, 2) / alpha))
						? Messages.getString("QuickCheckTableModel.Ok")
						: Messages.getString("QuickCheckTableModel.No"));
				sb.append("\n");
			}
		}
		return sb.toString();
	}

	static String costLabel(Design design) {
		return Messages.getString("DesignTablePane.Design_total_cost") + Tools.doubleFormat("#,##0", design.getCost());
	}

	// ------------------------------------------------------------ FrameNeatwork
	static String reportTopo(Topographie topo) {
		String s = "<html><head><title>";
		s += Messages.getString("FrameNeatwork.Topography_report");
		s += "</title></head><body>";
		s += ("<h1>" + Messages.getString("FrameNeatwork.Topography_report0") + topo.getName() + "</h1>");
		s += ("<h2>" + Messages.getString("FrameNeatwork.Node_List") + "</h2>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += Messages.getString(
				"FrameNeatwork.<tr><th>ID</th><th>Height</th><th>X</th><th>Y</th><th>Faucets</th><th>Nature</th></tr>");
		for (Iterator en = topo.getNodeIterator(); en.hasNext();) {
			Node n = (Node) en.next();
			s += "<tr>";
			s += ("<td>" + n.getName() + "</td>");
			s += ("<td>" + n.getHeight() + "</td>");
			s += ("<td>" + n.getCoordX() + "</td>");
			s += ("<td>" + n.getCoordY() + "</td>");
			s += ("<td>" + n.getNbTaps() + "</td>");
			s += ("<td>" + Node.getNameType(n.getType()) + "</td>");
			s += "</tr>";
		}
		s += "</table>";
		s += ("<h2>" + Messages.getString("FrameNeatwork.Arc_List") + "</h2>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += Messages.getString("FrameNeatwork.<tr><th>Begin</th><th>End</th><th>Length</th></tr>");
		for (Iterator en = topo.getPipeIterator(); en.hasNext();) {
			Pipe n = (Pipe) en.next();
			s += "<tr>";
			s += ("<td>" + n.getBegin() + "</td>");
			s += ("<td>" + n.getEnd() + "</td>");
			s += ("<td>" + n.getLength() + "</td>");
			s += "</tr>";
		}
		s += "</table>";
		s += ("<h2>" + Messages.getString("FrameNeatwork.Summary") + Messages.getString("FrameNeatwork.</h2>")); // $NON-NLS-3$
		s += (Messages.getString("FrameNeatwork.<B>Number_of_Nodes__</B>") + topo.getNbNodes() + "<UL>"
				+ Messages.getString("FrameNeatwork.<LI>_<i>Branching_nodes__</i>")
				+ topo.getNbNodes(Node.TYPE_DISPATCH)
				+ Messages.getString("FrameNeatwork.<LI>_<i>Faucet_nodes__</i>")
				+ topo.getNbNodes(Node.TYPE_FAUCET) + Messages.getString("FrameNeatwork._(with__")
				+ topo.getNbTotalTaps() + Messages.getString("FrameNeatwork._individual_faucets)_") + "</UL>"
				+ Messages.getString("FrameNeatwork.<B>Total_height_change__</B>")
				+ Tools.doubleFormat("#", topo.getTotalHeightChange()) + " m<BR><BR>"
				+ Messages.getString("FrameNeatwork.<B>Number_of_Pipes__</B>") + topo.getNbPipes() + " <BR>"
				+ Messages.getString("FrameNeatwork.<B>Total_length__</B>")
				+ Tools.doubleFormat("#", topo.getTotalLength()) + " m ");
		s += ("<p>" + "DATE" + "</p>");
		return s;
	}

	static String reportDesign(Design dsg) {
		String s = "<html><head><title>";
		s += Messages.getString("FrameNeatwork.Design_report");
		s += "</title></head><body>";
		s += ("<h1>" + Messages.getString("FrameNeatwork.Design_report0") + dsg.getName() + "</h1>");
		s += ("<h2>" + Messages.getString("FrameNeatwork.Node_List") + "</h2>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += Messages.getString(
				"FrameNeatwork.<tr><th>ID</th><th>Height</th><th>X</th><th>Y</th><th>Ideal_Orifice</th><th>Commercial_Orifice</th><th>Nature</th></tr>");
		for (Iterator en = dsg.getNodeIterator(); en.hasNext();) {
			Node n = (Node) en.next();
			s += "<tr>";
			s += ("<td>" + n.getName() + "</td>");
			s += ("<td>" + n.getHeight() + "</td>");
			s += ("<td>" + n.getCoordX() + "</td>");
			s += ("<td>" + n.getCoordY() + "</td>");
			s += ("<td>" + n.getOrifice() + "</td>");
			s += ("<td>" + n.getComercialOrifice() + "</td>");
			s += ("<td>" + Node.getNameType(n.getType()) + "</td>");
			s += "</tr>";
		}
		s += "</table>";
		s += ("<h2>" + Messages.getString("FrameNeatwork.Arc_List") + "</h2>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += Messages.getString(
				"FrameNeatwork.<tr><th>Begin</th><th>End</th><th>Length</th><th>Length1</th><th>Diam1</th><th>Length2</th><th>Diam2</th></tr>");
		for (Iterator en = dsg.getPipeIterator(); en.hasNext();) {
			Pipe n = (Pipe) en.next();
			s += "<tr>";
			s += ("<td>" + n.getBegin() + "</td>");
			s += ("<td>" + n.getEnd() + "</td>");
			s += ("<td>" + n.getLength() + "</td>");
			s += ("<td>" + n.getLength1() + "</td>");
			s += ("<td>" + n.getRefDiam1() + "</td>");
			s += ("<td>" + n.getLength2() + "</td>");
			s += ("<td>" + n.getRefDiam2() + "</td>");
			s += "</tr>";
		}
		s += "</table>";
		Hashtable usedlength = dsg.getSummary();
		s += ("<h2>" + Messages.getString("FrameNeatwork.Diameter_references") + "</h2>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += (Messages.getString(
				"FrameNeatwork.<tr><th>Ref</th><th>Nominal</th><th>SDR</th><th>Internal_Diameter</th><th>Unit_cost</th>")
				+ Messages.getString("FrameNeatwork.<th>Max_Pressure</th><th>Type</th><th>Roughness</th>")
				+ Messages.getString("FrameNeatwork.<th>Total_length</th><th>Total_cost</th></tr>"));
		Vector v = new Vector(dsg.getDiamTable().keySet());
		Collections.sort(v);
		for (Enumeration en = v.elements(); en.hasMoreElements();) {
			String k = en.nextElement().toString();
			Diameter d = (Diameter) dsg.getDiamTable().get(k);
			double l = ((usedlength.get(k) == null) ? 0 : Double.parseDouble(usedlength.get(k).toString()));
			s += "<tr>";
			s += ("<td>" + k + "</td>");
			s += ("<td>" + d.getNominal() + "</td>");
			s += ("<td>" + d.getSdr() + "</td>");
			s += ("<td>" + d.getDiameter() + "</td>");
			s += ("<td>" + Tools.doubleFormat("0.##", d.getCost()) + "</td>");
			s += ("<td>" + d.getMaxLength() + "</td>");
			s += ("<td>" + d.getType() + "</td>");
			s += ("<td>" + d.getRoughness() + "</td>");
			s += ("<td>" + l + "</td>");
			s += ("<td>" + (l * d.getCost()) + "</td>");
			s += "</tr>";
		}
		s += "</table>";
		if (dsg.getOrifices().size() > 0) {
			s += ("<h2>" + Messages.getString("FrameNeatwork.Available_orifices") + "</h2>"); // $NON-NLS-3$
			s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
			s += Messages.getString("FrameNeatwork.<tr><th>Diameter</th></tr>");
			for (Enumeration en = dsg.getOrifices().elements(); en.hasMoreElements();) {
				s += "<tr>";
				s += ("<td>" + en.nextElement().toString() + "</td>");
				s += "</tr>";
			}
			s += "</table>";
		}
		s += ("<h2>" + Messages.getString("FrameNeatwork.Summary") + "</h2>"); // $NON-NLS-3$
		s += ("<h3>" + Messages.getString("FrameNeatwork.Project_cost") + Tools.doubleFormat("0", dsg.getCost())
				+ "</h3>");
		Properties p = dsg.getProperties();
		s += ("<h3>" + Messages.getString("FrameNeatwork.Global_Parameter") + "</h3>"); // $NON-NLS-3$
		s += (Messages.getString("topo.watertemp.name") + " : " + p.getProperty("topo.watertemp.value") + "<br>");
		s += (Messages.getString("topo.pipelength.name") + " : " + p.getProperty("topo.pipelength.value") + "<br>");
		s += ("<h3>" + "Design Parameter" + "</h3>"); //$NON-NLS-3$
		s += (Messages.getString("topo.opentaps.name") + " : " + p.getProperty("topo.opentaps.value") + "<br>");
		s += (Messages.getString("topo.servicequal.name") + " : " + p.getProperty("topo.servicequal.value") + "<br>");
		s += (Messages.getString("topo.targetflow.name") + " : " + p.getProperty("topo.targetflow.value") + "<br>");
		s += (Messages.getString("topo.limitbudget.name") + " : " + p.getProperty("topo.limitbudget.value") + "<br>");
		s += ("<h3>" + Messages.getString("FrameNeatwork.Advanced_Parameter") + "</h3>"); // $NON-NLS-3$
		s += (Messages.getString("topo.orifcoef.name") + " : " + p.getProperty("topo.orifcoef.value") + "<br>");
		s += (Messages.getString("topo.faucetcoef.name") + " : " + p.getProperty("topo.faucetcoef.value") + "<br>");
		s += ("<h3>" + Messages.getString("FrameNeatwork.Structure") + "</h3>"); // $NON-NLS-3$
		s += (Messages.getString("FrameNeatwork.<B>Number_of_Nodes__</B>") + dsg.getNbNodes()
				+ Messages.getString("FrameNeatwork.<UL>_270")
				+ Messages.getString("FrameNeatwork.<LI>_<i>Branching_nodes__</i>")
				+ dsg.getNbNodes(Node.TYPE_DISPATCH)
				+ Messages.getString("FrameNeatwork.<LI>_<i>Faucet_nodes__</i>")
				+ dsg.getNbNodes(Node.TYPE_FAUCET) + Messages.getString("FrameNeatwork._(with")
				+ dsg.getNbTotalTaps() + " " + Messages.getString("FrameNeatwork._individual_faucets)")
				+ "</UL>" + Messages.getString("FrameNeatwork.<B>Total_height_change__</B>")
				+ Tools.doubleFormat("#", dsg.getTotalHeightChange()) + " m<BR><BR>"
				+ Messages.getString("FrameNeatwork.<B>Number_of_Pipes__</B>") + dsg.getNbPipes() + " <BR>"
				+ Messages.getString("FrameNeatwork.<B>Total_length__</B>")
				+ Tools.doubleFormat("#", dsg.getTotalLength()) + " m ");
		s += ("<p>" + "DATE" + "</p>");
		return s;
	}

	// ------------------------------------------------------------ SimulationPane
	static String reportSimu(Simulation simu, Design design) {
		String s = "<html><head><title>";
		s += Messages.getString("SimulationPane.Simulation_report");
		s += "</title></head><body>";
		s += ("<h1>" + Messages.getString("SimulationPane.Simulation_report0") + simu.getName() + "</h1>");
		s += ("<h2>" + Messages.getString("SimulationPane.Parameters") + "</h2>"); //$NON-NLS-3$
		s += (Messages.getString("SimulationPane.Simulated_design") + design.getName() + "<br>");
		s += (Messages.getString("SimulationPane.Type_of_simulation0")
				+ simu.getProperties().getProperty("simu.typesimu.value"));
		if (simu.getProperties().getProperty("simu.typesimu.value").equals("random")) {
			s += (Messages.getString("SimulationPane.<br>Number_of_simulations")
					+ simu.getProperties().getProperty("simu.nbsim.value"));
			s += (Messages.getString("SimulationPane.<br>Fraction_of_open_faucets")
					+ simu.getProperties().getProperty("simu.simopentaps.value"));
		}
		if (simu.getProperties().getProperty("simu.typesimu.value").equals("tapbytap")) {
			s += (Messages.getString("SimulationPane.<br>Number_of_simulations")
					+ simu.getProperties().getProperty("simu.nbsim.value"));
		}
		s += (Messages.getString("SimulationPane.<br>Used_type_of_orifice")
				+ simu.getProperties().getProperty("simu.typeorifice.value"));
		s += Messages.getString("SimulationPane.<h2>Flows_at_faucet</h2>");
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += (Messages.getString("SimulationPane.<tr><th>Faucet_ID</th><th>Nb_of_occurences</th><th>Min</th>")
				+ Messages.getString("SimulationPane.<th>Average</th><th>Max</th><th>Variability</th><th>&lt;")
				+ simu.getProperties().getProperty("simu.mincriticalflow.value", "") + "</th><th>&gt; "
				+ simu.getProperties().getProperty("simu.maxcriticalflow.value", "")
				+ Messages.getString("SimulationPane.</th><th>nb_of_failures</th></tr>"));
		for (Enumeration en = simu.getFlowTaps().elements(); en.hasMoreElements();) {
			Vector v = (Vector) en.nextElement();
			s += "<tr>";
			for (int i = 0; i < v.size(); i++) {
				s += ("<td>" + v.get(i) + "</td>");
			}
			s += "</tr>";
		}
		s += "</table>";
		s += Messages.getString("SimulationPane.<h2>Percentiles_at_faucet</h2>");
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += (Messages.getString("SimulationPane.<tr><th>Faucet_ID</th><th>Nb_of_occurences</th><th>Min</th>")
				+ Messages.getString("SimulationPane.<th>&lt;_10</th><th>&lt;_25</th><th>&lt;_50</th><th>&lt;_75</th>")
				+ Messages.getString("SimulationPane.<th>&lt;_90</th><th>Max</th></tr>"));
		for (Enumeration en = simu.getQuartileTaps().elements(); en.hasMoreElements();) {
			Vector v = (Vector) en.nextElement();
			s += "<tr>";
			for (int i = 0; i < v.size(); i++) {
				s += ("<td>" + v.get(i) + "</td>");
			}
			s += "</tr>";
		}
		s += "</table>";
		s += Messages.getString("SimulationPane.<h2>Speed_in_pipes</h2>");
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += (Messages.getString("SimulationPane.<tr><th>Pipe_ID</th><th>Nb_of_simulation</th><th>Average</th>")
				+ Messages.getString("SimulationPane.<th>Maximum</th></tr>"));
		for (Enumeration en = simu.getQuartileTaps().elements(); en.hasMoreElements();) {
			Vector v = (Vector) en.nextElement();
			s += "<tr>";
			for (int i = 0; i < 4; i++) {
				s += ("<td>" + v.get(i) + "</td>");
			}
			s += "</tr>";
		}
		s += "</table>";
		s += Messages.getString("SimulationPane.<h2>Nodes_Pressure</h2>");
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += (Messages.getString("SimulationPane.<tr><th>Node_ID</th><th>Minimum</th><th>Average</th>")
				+ Messages.getString("SimulationPane.<th>Maximum</th></tr>"));
		for (Enumeration en = simu.getNodesPressure().elements(); en.hasMoreElements();) {
			Vector v = (Vector) en.nextElement();
			s += "<tr>";
			for (int i = 0; i < v.size(); i++) {
				s += ("<td>" + v.get(i) + "</td>");
			}
			s += "</tr>";
		}
		s += "</table>";
		return s;
	}

	// ------------------------------------------------------------ DatabaseDialog
	static String dbDiameters(Database database) {
		String s = Messages.getString("DatabaseDialog.<html><head><title>");
		s += Messages.getString("DatabaseDialog.Database_-_Diameters");
		s += "</title></head><body>";
		s += ("<h1>" + Messages.getString("DatabaseDialog.Database_-_Diameters") + "</h1>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += Messages.getString(
				"DatabaseDialog.<tr><th>Nominal</th><th>SDR</th><th>Cost</th><th>Max_Pressure</th><th>Type</th><th>Roughness</th></tr>");
		for (Enumeration en = database.getDiameters(); en.hasMoreElements();) {
			Diameter d = (Diameter) en.nextElement();
			s += "<tr>";
			s += ("<td>" + d.getNominal() + "</td>");
			s += ("<td>" + d.getSdr() + "</td>");
			s += ("<td>" + d.getCost() + "</td>");
			s += ("<td>" + d.getMaxLength() + "</td>");
			s += ("<td>" + d.getType() + "</td>");
			s += ("<td>" + d.getRoughness() + "</td>");
			s += "</tr>";
		}
		s += ("</table><p>" + "DATE" + "</p>");
		return s;
	}

	static String dbOrifices(Database database) {
		String s = "<html><head><title>";
		s += Messages.getString("DatabaseDialog.Database_-_Orifices");
		s += "</title></head><body>";
		s += ("<h1>" + Messages.getString("DatabaseDialog.Database_-_Orifices") + "</h1>"); // $NON-NLS-3$
		s += "<table BORDER CELLPADDING=0 CELLSPACING=0>";
		s += Messages.getString("DatabaseDialog.<tr><th>Diameter</th></tr>");
		for (Iterator en = database.getOrifices().iterator(); en.hasNext();) {
			Orifice d = (Orifice) en.next();
			s += "<tr>";
			s += ("<td>" + d.getDiameter() + "</td>");
			s += "</tr>";
		}
		s += ("</table><p>" + "DATE" + "</p>");
		return s;
	}

	// ------------------------------------------------------------ ProjectManager / AbstractFileManager
	static String isAlreadyLoaded(String projectName, List<String> names) {
		String oldname = projectName;
		String name = oldname;
		int cpt = 2;
		int i = oldname.lastIndexOf("(");
		int j = oldname.lastIndexOf(")");
		if (i < j) {
			try {
				cpt = Integer.parseInt(oldname.substring(i + 1, j));
				oldname = oldname.substring(0, i - 1);
				name = oldname + " (" + String.valueOf(cpt++) + ")";
			} catch (NumberFormatException e) {
			}
		}
		while (names.contains(name))
			name = oldname + " (" + String.valueOf(cpt++) + ")";
		return name;
	}

	static String getFirstNameCompatible(String pname, String[] f) {
		String name = pname;
		String oldName = name;
		int cpt = 2;
		while (Arrays.asList(f).contains(name))
			name = oldName + " (" + String.valueOf(cpt++) + ")";
		return name;
	}
}
