//! Texts of the interface, read from the Java message bundles of NeatWork
//! 2024 (`neatwork_*.properties`), as `neatwork.Messages` does.
//!
//! The Rust interface adds a few features (the network drawing). Their
//! texts are in [`EXTRA`], with keys that start with `Gui.`.

use std::collections::HashMap;

use neatwork_core::java::JProperties;

/// Language index used by the `T` global of the `.slint` files.
pub const EN: i32 = 0;
pub const FR: i32 = 1;
pub const SP: i32 = 2;

const BASE: &[u8] = include_bytes!("../resources/neatwork.properties");
const BUNDLE_EN: &[u8] = include_bytes!("../resources/neatwork_en.properties");
const BUNDLE_FR: &[u8] = include_bytes!("../resources/neatwork_fr.properties");
const BUNDLE_SP: &[u8] = include_bytes!("../resources/neatwork_sp.properties");

/// Texts of the features that the Java program does not have:
/// (key, English, French, Spanish).
const EXTRA: &[(&str, &str, &str, &str)] = &[
    ("Gui.Network", "Network", "Réseau", "Red"),
    ("Gui.Layout", "Layout", "Disposition", "Disposición"),
    ("Gui.Layout_tree", "Tree", "Arbre", "Árbol"),
    ("Gui.Layout_profile", "Profile", "Profil", "Perfil"),
    (
        "Gui.Layout_coordinates",
        "Coordinates",
        "Coordonnées",
        "Coordenadas",
    ),
    ("Gui.Color", "Color", "Couleur", "Color"),
    ("Gui.Zoom", "Zoom", "Zoom", "Zoom"),
    ("Gui.Fit", "Fit", "Ajuster", "Ajustar"),
    ("Gui.Legend", "Legend", "Légende", "Leyenda"),
    (
        "Gui.Color_type",
        "Node type",
        "Type de nœud",
        "Tipo de nodo",
    ),
    ("Gui.Color_height", "Height", "Altitude", "Altura"),
    (
        "Gui.Color_diameter",
        "Pipe diameter",
        "Diamètre des tuyaux",
        "Diámetro de los tubos",
    ),
    (
        "Gui.Color_flow",
        "Mean flow at faucets",
        "Débit moyen aux robinets",
        "Caudal medio en los grifos",
    ),
    (
        "Gui.Color_pressure",
        "Mean pressure",
        "Pression moyenne",
        "Presión media",
    ),
    (
        "Gui.No_coordinates",
        "The nodes have no coordinates: tree layout shown.",
        "Les nœuds n'ont pas de coordonnées : disposition en arbre.",
        "Los nodos no tienen coordenadas: disposición en árbol.",
    ),
    (
        "Gui.No_simulation",
        "Select a simulation in the Simulation tab.",
        "Choisissez une simulation dans l'onglet Simulation.",
        "Elija una simulación en la pestaña Simulación.",
    ),
    (
        "Gui.Path_length",
        "Distance to the source",
        "Distance à la source",
        "Distancia a la fuente",
    ),
    ("Gui.Faucets", "Faucets", "Robinets", "Grifos"),
    ("Gui.Error", "Error", "Erreur", "Error"),
    (
        "Gui.Low",
        "below the low critical flow",
        "sous le débit critique bas",
        "bajo el caudal crítico bajo",
    ),
    (
        "Gui.Ok_flow",
        "between the critical flows",
        "entre les débits critiques",
        "entre los caudales críticos",
    ),
    (
        "Gui.High",
        "above the high critical flow",
        "au-dessus du débit critique haut",
        "sobre el caudal crítico alto",
    ),
];

/// The bundles of the three languages, with the base bundle as fallback.
pub struct Messages {
    bundles: [HashMap<String, String>; 3],
    base: HashMap<String, String>,
}

/// Decode a `.properties` file as `PropertyResourceBundle` does: UTF-8,
/// or ISO-8859-1 when the bytes are not valid UTF-8.
fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|b| char::from(*b)).collect(),
    }
}

fn load(bytes: &[u8]) -> HashMap<String, String> {
    let mut p = JProperties::new();
    p.load(&decode(bytes));
    p.entries().map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// Remove HTML tags; turn line-breaking tags into new lines.
pub fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut tag = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match (in_tag, c) {
            (false, '<') => {
                in_tag = true;
                tag.clear();
            }
            (true, '>') => {
                in_tag = false;
                let t = tag.trim_start_matches('/').to_ascii_lowercase();
                let name = t.split_whitespace().next().unwrap_or("");
                if matches!(
                    name,
                    "br" | "p" | "h1" | "h2" | "h3" | "li" | "tr" | "ul" | "blockquote"
                ) && !out.ends_with('\n')
                    && !out.is_empty()
                {
                    out.push('\n');
                }
                if name == "li" && !tag.starts_with('/') {
                    out.push_str("  - ");
                }
            }
            (true, _) => tag.push(c),
            (false, _) => out.push(c),
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

impl Messages {
    pub fn new() -> Self {
        let mut bundles = [load(BUNDLE_EN), load(BUNDLE_FR), load(BUNDLE_SP)];
        for (k, en, fr, sp) in EXTRA {
            bundles[0].insert((*k).to_string(), (*en).to_string());
            bundles[1].insert((*k).to_string(), (*fr).to_string());
            bundles[2].insert((*k).to_string(), (*sp).to_string());
        }
        Messages {
            bundles,
            base: load(BASE),
        }
    }

    /// Raw text of a key (with HTML), `!key!` when it is missing, as
    /// `Messages.getString` returns.
    pub fn raw(&self, lang: i32, key: &str) -> String {
        let b = &self.bundles[usize::try_from(lang).unwrap_or(0).min(2)];
        b.get(key)
            .or_else(|| self.base.get(key))
            .cloned()
            .unwrap_or_else(|| format!("!{key}!"))
    }

    /// Text for display: HTML removed, surrounding new lines trimmed.
    pub fn get(&self, lang: i32, key: &str) -> String {
        let raw = self.raw(lang, key);
        if raw.contains('<') {
            strip_html(&raw).trim().to_string()
        } else {
            raw.trim_end_matches('\n').to_string()
        }
    }
}

/// Code of a language in the file `neatwork.locale` (`appli.locale`).
pub fn locale_code(lang: i32) -> &'static str {
    match lang {
        FR => "fr",
        SP => "sp",
        _ => "en",
    }
}

pub fn lang_from_code(code: &str) -> i32 {
    match code {
        "fr" => FR,
        "sp" | "es" => SP,
        _ => EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles_load() {
        let m = Messages::new();
        assert_eq!(m.get(EN, "FrameNeatwork.File"), "File");
        assert_eq!(
            m.get(FR, "topo.watertemp.name"),
            "Température de l'eau (°C)"
        );
        assert_eq!(m.get(SP, "Network.Source"), "Tanque");
        assert_eq!(m.get(EN, "nope"), "!nope!");
        assert_eq!(m.get(EN, "Gui.Network"), "Network");
        assert!(!m.get(EN, "PresentationPanel.text").contains('<'));
    }
}
