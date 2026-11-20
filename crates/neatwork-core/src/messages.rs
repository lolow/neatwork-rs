//! The localized strings that reach NeatWork output files. They come from
//! the Java bundles `neatwork_en.properties`, `neatwork_fr.properties` and
//! `neatwork_sp.properties`.

/// Language of the localized strings written to files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    En,
    Fr,
    /// Spanish. The Java bundle suffix is `sp`.
    Sp,
}

impl Locale {
    /// Parse `en`, `fr`, `sp` or `es`.
    pub fn parse(s: &str) -> Option<Locale> {
        match s {
            "en" => Some(Locale::En),
            "fr" => Some(Locale::Fr),
            "sp" | "es" => Some(Locale::Sp),
            _ => None,
        }
    }

    /// `MakeSimulation.Default_properties`, header of the `.sim` parameters.
    pub fn simulation_properties_header(self) -> &'static str {
        match self {
            Locale::En => "!Default properties\n",
            Locale::Fr => "!Propri\u{e9}t\u{e9}s par d\u{e9}faut\n",
            Locale::Sp => "!Propiedades del sistema no registradas\n",
        }
    }

    /// `MakeSimulation.Global_average`, first row of the flow table.
    pub fn global_average(self) -> &'static str {
        match self {
            Locale::En => "Global average",
            Locale::Fr => "Moyenne totale",
            Locale::Sp => "Media global",
        }
    }

    /// `Network.Source`, name of a source the program adds itself.
    pub fn source_name(self) -> &'static str {
        match self {
            Locale::En | Locale::Fr => "Source",
            Locale::Sp => "Tanque",
        }
    }

    /// `Project.error`, prefix of a load error.
    pub fn error_prefix(self) -> &'static str {
        match self {
            Locale::En | Locale::Sp => "error:",
            Locale::Fr => "erreur:",
        }
    }

    /// `Project.warning`, prefix of a load warning.
    pub fn warning_prefix(self) -> &'static str {
        match self {
            Locale::En => "warning:",
            Locale::Fr => "attention:",
            Locale::Sp => "advertencia:",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles() {
        assert_eq!(Locale::parse("es"), Some(Locale::Sp));
        assert_eq!(
            Locale::Fr.simulation_properties_header(),
            "!Propriétés par défaut\n"
        );
        assert_eq!(Locale::Sp.source_name(), "Tanque");
        assert_eq!(Locale::En.global_average(), "Global average");
    }
}
