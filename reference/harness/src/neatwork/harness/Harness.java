package neatwork.harness;

import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Enumeration;
import java.util.Hashtable;
import java.util.Iterator;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.StringTokenizer;
import java.util.Vector;

import neatwork.Messages;
import neatwork.file.AbstractFileManager;
import neatwork.project.Database;
import neatwork.project.Design;
import neatwork.project.Diameter;
import neatwork.project.Node;
import neatwork.project.Orifice;
import neatwork.project.Topographie;
import neatwork.solver.SolverDisk;

/**
 * Headless driver for the NeatWork 2024 engine. It repeats the calls that the
 * Swing GUI makes (FrameNeatwork, MakeDesignDialog, NewSimuDialog,
 * SimulationPane) without opening a window.
 *
 * Usage:
 *   design   <in.tpo> <out.dsg>
 *   simulate <in.dsg> <out.sim> [--type random|tapbytap|handmade]
 *            [--nbsim N] [--opentaps R] [--orifice commercial|ideal]
 *            [--open TAP1,TAP2,...]
 *
 * System properties: nw.lp.dump, nw.lp.solution (see the stub Solver).
 */
public class Harness {

	public static void main(String[] args) throws Exception {
		Locale.setDefault(Locale.ENGLISH);
		Messages.setLocale(Locale.ENGLISH);
		if (args.length < 3) {
			System.err.println("usage: design <in.tpo> <out.dsg> | simulate <in.dsg> <out.sim> [options]");
			System.exit(2);
		}
		Properties app = loaderProperties();
		switch (args[0]) {
		case "design":
			design(app, Path.of(args[1]), Path.of(args[2]));
			break;
		case "simulate":
			simulate(app, Path.of(args[1]), Path.of(args[2]), args);
			break;
		default:
			System.err.println("unknown command " + args[0]);
			System.exit(2);
		}
	}

	/** Same properties as Loader.main builds for the standalone app. */
	static Properties loaderProperties() throws IOException {
		Properties p = new Properties();
		try (InputStream in = Harness.class.getResourceAsStream("/neatwork/default.properties")) {
			p.load(in);
		}
		p.setProperty("appli.standalone", "true");
		p.setProperty("solver.distant", "false");
		p.setProperty("file.distant", "false");
		p.setProperty("file.path", ".");
		Properties po = new Properties();
		po.setProperty("appli.locale", "en");
		p.putAll(po);
		return p;
	}

	static String projectName(Path file) {
		String n = file.getFileName().toString();
		int dot = n.lastIndexOf('.');
		return dot > 0 ? n.substring(0, dot) : n;
	}

	static void design(Properties app, Path in, Path out) throws IOException {
		String txt = Files.readString(in, StandardCharsets.UTF_8);
		Topographie topo = new Topographie(projectName(in), txt, app);
		if (topo.getInfoModif().contains(neatwork.project.Project.ERROR)) {
			System.err.print(topo.getInfoModif());
		}

		// MakeDesignDialog constructor, then FrameNeatwork restores the heights
		double hsource = topo.setHauteurSource(0);
		topo.makeExpandedTopo();
		topo.setHauteurSource(hsource);

		// LoadFactorTableModel.updateData
		Hashtable loadFactors = topo.getLoadFactor(topo.getLoadTaps());
		Hashtable modifLoadFactor = new Hashtable(loadFactors);

		// ChooseMaterialPane with every row selected
		Database db = new Database(new NoFiles(), app);
		StringBuilder diam = new StringBuilder();
		for (Enumeration e = db.getDiameters(); e.hasMoreElements();) {
			Diameter d = (Diameter) e.nextElement();
			diam.append(d.getNominal()).append(',').append(d.getSdr()).append(',').append(d.getDiameter())
					.append(',').append(d.getCost()).append(',').append(d.getMaxLength()).append(',')
					.append(d.getType()).append(',').append(d.getRoughness()).append('\n');
		}
		StringBuilder orif = new StringBuilder();
		for (Object o : (List) db.getOrifices()) {
			orif.append(((Orifice) o).getDiameter()).append('\n');
		}

		SolverDisk solver = new SolverDisk();
		solver.init();
		solver.makeDesign(topo, orif.toString(), diam.toString(), modifLoadFactor, new Vector(), hsource);

		// FrameNeatwork.runMakeDesign
		Design design = new Design(topo.getName(), solver.getDesignContentMakeDesign(), app);
		design.setProperties(topo.getProperties());
		design.setLoadFactorTable(modifLoadFactor);
		writeProjectFile(out, design.getContent());
	}

	static void simulate(Properties app, Path in, Path out, String[] args) throws IOException {
		String type = "random";
		String nbsim = app.getProperty("simu.nbsim.value");
		String opentaps = app.getProperty("simu.simopentaps.value");
		String orifice = app.getProperty("simu.typeorifice.value");
		String open = "";
		for (int i = 3; i < args.length; i++) {
			switch (args[i]) {
			case "--type": type = args[++i]; break;
			case "--nbsim": nbsim = args[++i]; break;
			case "--opentaps": opentaps = args[++i]; break;
			case "--orifice": orifice = args[++i]; break;
			case "--open": open = args[++i]; break;
			default:
				System.err.println("unknown option " + args[i]);
				System.exit(2);
			}
		}
		String txt = Files.readString(in, StandardCharsets.UTF_8);
		Design design = new Design(projectName(in), txt, app);
		// The GUI swaps each diameter for the equal database entry on load.
		design.refreshDesign(new Database(new NoFiles(), app));

		Hashtable faucetRef = new Hashtable();
		if (!type.equals("random")) {
			int faucets = 0;
			for (Iterator it = design.getNodeIterator(); it.hasNext();) {
				Node n = (Node) it.next();
				if (n.getType() == Node.TYPE_FAUCET) {
					faucets++;
					if (type.equals("handmade")) {
						faucetRef.put(n.getName(), Boolean.FALSE);
					}
				}
			}
			opentaps = "-";
			nbsim = type.equals("tapbytap") ? "" + faucets : "1";
		}
		if (type.equals("handmade")) {
			StringTokenizer st = new StringTokenizer(open, ",");
			while (st.hasMoreTokens()) {
				faucetRef.put(st.nextToken(), Boolean.TRUE);
			}
		}

		// NewSimuDialog(design.getProperties()) and getParameters()
		Properties prop = new Properties(design.getProperties());
		prop.putAll(design.getProperties());
		Properties p = new Properties(prop);
		p.setProperty("simu.nbsim.value", nbsim);
		p.setProperty("simu.simopentaps.value", opentaps);
		p.setProperty("simu.targetflow.value", prop.getProperty("simu.targetflow.value"));
		p.setProperty("simu.mincriticalflow.value", prop.getProperty("simu.mincriticalflow.value"));
		p.setProperty("simu.maxcriticalflow.value", prop.getProperty("simu.maxcriticalflow.value"));
		p.setProperty("simu.typeorifice.value", orifice);
		p.setProperty("simu.typesimu.value", type);

		SolverDisk solver = new SolverDisk();
		solver.init();
		solver.simulation(design, p, faucetRef);
		writeProjectFile(out, solver.getSimulationContent());
	}

	/** FileManagerDisk.writeFile: drop empty lines, end each line with a newline. */
	static void writeProjectFile(Path out, String content) throws IOException {
		try (BufferedWriter f = Files.newBufferedWriter(out, StandardCharsets.UTF_8)) {
			StringTokenizer st = new StringTokenizer(content, "\n\r");
			while (st.hasMoreTokens()) {
				f.write(st.nextToken());
				f.write('\n');
			}
		}
	}

	/** File manager without files: Database falls back to its classpath resources. */
	static class NoFiles extends AbstractFileManager {
		public String[] getListFile(int type) { return new String[0]; }
		public String readFile(String fileName, int type) { return ""; }
		public boolean writeFile(String fileName, String content, int type) { return false; }
		public boolean deleteFile(String fileName, int type) { return false; }
		public String readDbFile(String fileName) { return ""; }
		public boolean writeDbFile(String fileName, String content) { return false; }
	}
}
