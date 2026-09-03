use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppLanguage {
    PortugueseBrazil,
    English,
}

impl AppLanguage {
    pub const fn all() -> [Self; 2] {
        [Self::PortugueseBrazil, Self::English]
    }

    pub const fn locale(self) -> &'static str {
        match self {
            Self::PortugueseBrazil => "pt-BR",
            Self::English => "en-US",
        }
    }

    pub const fn native_name(self) -> &'static str {
        match self {
            Self::PortugueseBrazil => "Português (Brasil)",
            Self::English => "English",
        }
    }

    pub const fn resource(self) -> &'static str {
        match self {
            Self::PortugueseBrazil => include_str!("locales/pt-BR.ftl"),
            Self::English => include_str!("locales/en-US.ftl"),
        }
    }

    pub fn from_setting(value: &str) -> Option<Self> {
        match value.trim() {
            "pt-BR" => Some(Self::PortugueseBrazil),
            "en-US" => Some(Self::English),
            _ => None,
        }
    }

    pub fn from_os_locale(locale: Option<&str>) -> Self {
        let normalized = locale.unwrap_or_default().to_ascii_lowercase();
        if normalized == "pt" || normalized.starts_with("pt-") || normalized.starts_with("pt_") {
            Self::PortugueseBrazil
        } else {
            Self::English
        }
    }
}

impl fmt::Display for AppLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.native_name())
    }
}
