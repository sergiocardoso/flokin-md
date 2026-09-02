mod language;

use chrono::{DateTime, Local, Utc};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
pub use language::AppLanguage;
use unic_langid::LanguageIdentifier;

pub struct I18nCatalog {
    language: AppLanguage,
    active: FluentBundle<FluentResource>,
    fallback: FluentBundle<FluentResource>,
}

impl I18nCatalog {
    pub fn new(language: AppLanguage) -> Self {
        Self {
            language,
            active: bundle(language),
            fallback: bundle(AppLanguage::English),
        }
    }

    pub fn tr(&self, key: &str) -> String {
        self.tr_with(key, &[])
    }

    pub fn tr_static(&self, key: &str) -> &'static str {
        match (self.language, key) {
            (AppLanguage::PortugueseBrazil, "search-placeholder") => "Buscar documentos...",
            (AppLanguage::English, "search-placeholder") => "Search documents...",
            (AppLanguage::PortugueseBrazil, "health-filter-placeholder") => "Filtrar issues...",
            (AppLanguage::English, "health-filter-placeholder") => "Filter issues...",
            (AppLanguage::PortugueseBrazil, "editor-empty-file") => "Arquivo vazio.",
            (AppLanguage::English, "editor-empty-file") => "Empty file.",
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Missing static translation key: {key}");
                ""
            }
        }
    }

    pub fn tr_with(&self, key: &str, args: &[(&str, FluentArgValue<'_>)]) -> String {
        let fluent_args = args
            .iter()
            .fold(FluentArgs::new(), |mut fluent_args, (name, value)| {
                match value {
                    FluentArgValue::Str(value) => fluent_args.set(*name, FluentValue::from(*value)),
                    FluentArgValue::Owned(value) => {
                        fluent_args.set(*name, FluentValue::from(value.as_str()))
                    }
                    FluentArgValue::Number(value) => {
                        fluent_args.set(*name, FluentValue::from(*value))
                    }
                }
                fluent_args
            });
        self.format_from(&self.active, key, Some(&fluent_args))
            .or_else(|| self.format_from(&self.fallback, key, Some(&fluent_args)))
            .unwrap_or_else(|| {
                #[cfg(debug_assertions)]
                eprintln!("Missing translation key: {key}");
                key.to_owned()
            })
    }

    pub fn format_datetime(&self, timestamp: DateTime<Utc>) -> String {
        let local = timestamp.with_timezone(&Local);
        match self.language {
            AppLanguage::PortugueseBrazil => local.format("%d/%m/%Y %H:%M").to_string(),
            AppLanguage::English => local.format("%b %-d, %Y %-I:%M %p").to_string(),
        }
    }

    fn format_from(
        &self,
        bundle: &FluentBundle<FluentResource>,
        key: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Option<String> {
        let message = bundle.get_message(key)?;
        let pattern = message.value()?;
        let mut errors = Vec::new();
        Some(
            bundle
                .format_pattern(pattern, args, &mut errors)
                .into_owned(),
        )
    }
}

impl std::fmt::Debug for I18nCatalog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("I18nCatalog")
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub enum FluentArgValue<'a> {
    Str(&'a str),
    Owned(String),
    Number(i64),
}

impl<'a> From<&'a str> for FluentArgValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(value)
    }
}

impl From<String> for FluentArgValue<'_> {
    fn from(value: String) -> Self {
        Self::Owned(value)
    }
}

impl From<usize> for FluentArgValue<'_> {
    fn from(value: usize) -> Self {
        Self::Number(value as i64)
    }
}

impl From<i64> for FluentArgValue<'_> {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

fn bundle(language: AppLanguage) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = language
        .locale()
        .parse()
        .expect("registered app locale must be valid");
    let resource = FluentResource::try_new(language.resource().to_owned())
        .expect("embedded Fluent resource must parse");
    let mut bundle = FluentBundle::new(vec![langid]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .expect("embedded Fluent resource has duplicate keys");
    bundle
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn os_locale_detection_maps_portuguese_to_pt_br() {
        assert_eq!(
            AppLanguage::from_os_locale(Some("pt-BR")),
            AppLanguage::PortugueseBrazil
        );
        assert_eq!(
            AppLanguage::from_os_locale(Some("pt_PT")),
            AppLanguage::PortugueseBrazil
        );
        assert_eq!(
            AppLanguage::from_os_locale(Some("en-US")),
            AppLanguage::English
        );
        assert_eq!(AppLanguage::from_os_locale(None), AppLanguage::English);
    }

    #[test]
    fn locale_files_have_matching_keys() {
        let pt = keys(AppLanguage::PortugueseBrazil.resource());
        let en = keys(AppLanguage::English.resource());
        assert_eq!(pt, en);
    }

    #[test]
    fn plural_messages_use_active_language() {
        let pt = I18nCatalog::new(AppLanguage::PortugueseBrazil);
        let en = I18nCatalog::new(AppLanguage::English);
        assert_eq!(
            pt.tr_with("files-restored", &[("count", 1usize.into())]),
            "1 arquivo restaurado."
        );
        assert_eq!(
            en.tr_with("files-restored", &[("count", 2usize.into())]),
            "2 files restored."
        );
    }

    fn keys(resource: &str) -> BTreeSet<String> {
        resource
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, _)| key.trim().to_owned())
            .filter(|key| !key.is_empty() && !key.starts_with('#'))
            .collect()
    }
}
