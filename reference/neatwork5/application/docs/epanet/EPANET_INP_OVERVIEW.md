# EPANET INP study

Primary reference:

- EPANET 2.2 User Manual
- U.S. Environmental Protection Agency
- Appendix C — Input File Format

Target compatibility:

EPANET 2.2 `.inp`

Secondary references:

- EPANET 2.2 Source Code (EPA GitHub release)
- EPANET 2.2 Input/Output File Format documentation
- EPA Water Research — EPANET page

Scope of this document:

Describe the structure of an EPANET 2.2 `.inp` file, its sections,
its lexical rules, and the parser policy that will be adopted in
Step 11.

This document does NOT describe the NeatWork mapping. That is in
`EPANET_INP_MAPPING.md`.

---
## 1. What is a `.inp` file?

An EPANET `.inp` file is a **section-based plain text file**.

Key lexical facts:

- The file is composed of **sections**.
- A section starts with a header line of the form `[SECTION_NAME]`.
- Section names are case-insensitive in practice but conventionally
  written in uppercase.
- The file is **order-sensitive only within a section context**, not
  at file level (sections may appear in any order, but references must
  resolve after the file has been fully read).
- `;` starts a comment. Everything after `;` on a line is a comment.
- Blank lines are allowed and ignored.
- Lines may be terminated by `\n` or `\r\n`.
- The file ends with `[END]`, but parsers should not require it.
- Column separators are whitespace (spaces and/or tabs).
- Some fields are quoted strings (e.g. in `[TITLE]`), most are not.

Reference: EPANET 2.2 User Manual, Appendix C.

---

## 2. Section groups

| Group | Important sections |
|---|---|
| Network | `[TITLE]`, `[JUNCTIONS]`, `[RESERVOIRS]`, `[TANKS]`, `[PIPES]`, `[PUMPS]`, `[VALVES]`, `[EMITTERS]` |
| Operation | `[CURVES]`, `[PATTERNS]`, `[ENERGY]`, `[STATUS]`, `[CONTROLS]`, `[RULES]`, `[DEMANDS]` |
| Water quality | `[QUALITY]`, `[REACTIONS]`, `[SOURCES]`, `[MIXING]` |
| Analysis | `[OPTIONS]`, `[TIMES]`, `[REPORT]` |
| Map | `[COORDINATES]`, `[VERTICES]`, `[LABELS]`, `[BACKDROP]`, `[TAGS]` |

This grouping follows the EPANET 2.2 User Manual.


| EPANET | NeatWork | result |
|---|---|---|
| Junction ID | `nodes.noeud` | ✅ directly |
| Elevation | `nodes.haut` | ✅ directly after Unit conversion |
| Coordinates | `nodes.xdata/ydata` | ✅ from `[COORDINATES]` |
| Base Demand | `clients.cible` | 🟡 requires Adapter |
| Pattern | Currently has no equivalent | 🟡 Preserve / future |

---

## 3. Parser policy

The parser (Step 11) must follow these rules:

```text
Unknown section
→ do not crash unnecessarily
→ preserve raw section verbatim
→ add a WARNING to the import report

Unknown keyword inside a known section
→ preserve raw line
→ add a WARNING

Malformed line (wrong column count)
→ preserve raw line
→ add a WARNING
→ do not throw

Duplicate entity ID
→ add a WARNING
→ deterministic resolution policy (first wins, or reject)
→ must be decided explicitly in Step 11