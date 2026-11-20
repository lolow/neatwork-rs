package neatwork.solver;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

import mosek.boundkey;

/**
 * Harness replacement for neatwork.solver.Solver.
 *
 * The original calls MOSEK through its Java bindings, which have no Linux
 * native library here. This stub keeps the signature of lp() and:
 *  - writes the LP that the Java code built to the file named by the system
 *    property nw.lp.dump (JSON, MOSEK column-major layout);
 *  - fills x from the file named by nw.lp.solution (one number per line),
 *    so the Java post-processing runs on a known LP solution.
 * When no solution file is given, x stays at zero, which is what the
 * original does when MOSEK reports a non-optimal status.
 */
public class Solver {

	public void lp(int numcon, int numvar, int numanz, boundkey[] bkc, double[] blc, double[] buc, boundkey[] bkx,
			double[] blx, double[] bux, int[] ptrb, int[] ptre, int[] asub, double[] aval, double[] x, double[] c) {
		String dump = System.getProperty("nw.lp.dump");
		if (dump != null) {
			StringBuilder s = new StringBuilder();
			s.append("{\n");
			s.append("\"numcon\":").append(numcon).append(",\n");
			s.append("\"numvar\":").append(numvar).append(",\n");
			s.append("\"numanz\":").append(numanz).append(",\n");
			s.append("\"bkc\":").append(keys(bkc)).append(",\n");
			s.append("\"blc\":").append(nums(blc)).append(",\n");
			s.append("\"buc\":").append(nums(buc)).append(",\n");
			s.append("\"bkx\":").append(keys(bkx)).append(",\n");
			s.append("\"blx\":").append(nums(blx)).append(",\n");
			s.append("\"bux\":").append(nums(bux)).append(",\n");
			s.append("\"ptrb\":").append(ints(ptrb)).append(",\n");
			s.append("\"ptre\":").append(ints(ptre)).append(",\n");
			s.append("\"asub\":").append(ints(asub)).append(",\n");
			s.append("\"aval\":").append(nums(aval)).append(",\n");
			s.append("\"c\":").append(nums(c)).append("\n");
			s.append("}\n");
			try {
				Files.writeString(Path.of(dump), s.toString(), StandardCharsets.UTF_8);
			} catch (IOException e) {
				throw new UncheckedIOException(e);
			}
		}
		String sol = System.getProperty("nw.lp.solution");
		if (sol != null) {
			try {
				String[] lines = Files.readString(Path.of(sol), StandardCharsets.UTF_8).trim().split("\\s+");
				for (int j = 0; j < numvar; j++) {
					x[j] = Double.parseDouble(lines[j]);
				}
			} catch (IOException e) {
				throw new UncheckedIOException(e);
			}
		}
	}

	public void nlp(int cont, int numvar, int numanz, boundkey[] bkc, double[] blc, double[] buc, boundkey[] bkx,
			int[] ptrb, int[] ptre, double[] blx, double[] bux, double[] x, double[] y, double[] c, int[] sub,
			double[] val, double[] oprfo, double[] oprgo, double[] oprho, mosek.scopr[] opro, int[] oprjo) {
		throw new UnsupportedOperationException("nlp is dead code in NeatWork 2024");
	}

	private static String keys(boundkey[] k) {
		StringBuilder s = new StringBuilder("[");
		for (int i = 0; i < k.length; i++) {
			if (i > 0) s.append(',');
			s.append('"').append(k[i]).append('"');
		}
		return s.append(']').toString();
	}

	private static String nums(double[] d) {
		StringBuilder s = new StringBuilder("[");
		for (int i = 0; i < d.length; i++) {
			if (i > 0) s.append(',');
			s.append(Double.toString(d[i]));
		}
		return s.append(']').toString();
	}

	private static String ints(int[] d) {
		StringBuilder s = new StringBuilder("[");
		for (int i = 0; i < d.length; i++) {
			if (i > 0) s.append(',');
			s.append(d[i]);
		}
		return s.append(']').toString();
	}
}
