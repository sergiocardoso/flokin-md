use std::path::PathBuf;

use crate::{Document, PropertyValue};

pub const DEFAULT_SEARCH_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub limit: usize,
}

impl SearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: DEFAULT_SEARCH_LIMIT,
        }
    }

    pub fn with_limit(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            limit,
        }
    }

    fn terms(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .map(fold_case)
            .filter(|term| !term.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchState {
    pub open: bool,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: Option<usize>,
    pub total_matches: usize,
}

impl SearchState {
    pub fn closed() -> Self {
        Self {
            open: false,
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            total_matches: 0,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        if !self.results.is_empty() {
            self.selected_index =
                Some(self.selected_index.unwrap_or(0).min(self.results.len() - 1));
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.selected_index = None;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = None;
    }

    pub fn apply_outcome(&mut self, outcome: SearchOutcome) {
        self.total_matches = outcome.total_matches;
        self.results = outcome.results;
        self.selected_index = if self.results.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn select_next(&mut self) {
        if self.results.is_empty() {
            self.selected_index = None;
            return;
        }

        self.selected_index = Some(match self.selected_index {
            Some(index) => (index + 1).min(self.results.len() - 1),
            None => 0,
        });
    }

    pub fn select_previous(&mut self) {
        if self.results.is_empty() {
            self.selected_index = None;
            return;
        }

        self.selected_index = Some(match self.selected_index {
            Some(index) => index.saturating_sub(1),
            None => 0,
        });
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.selected_index
            .and_then(|index| self.results.get(index))
    }

    pub fn is_limited(&self) -> bool {
        self.total_matches > self.results.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOutcome {
    pub results: Vec<SearchResult>,
    pub total_matches: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub document_path: PathBuf,
    pub title: String,
    pub relative_path: PathBuf,
    pub score: u32,
    pub matched_field: SearchMatchedField,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchMatchedField {
    Title,
    FileName,
    RelativePath,
    Frontmatter,
    Content,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    result: SearchResult,
    sort_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TermMatch {
    score: u32,
    field: SearchMatchedField,
    snippet: Option<String>,
}

pub fn search_documents(query: SearchQuery, documents: &[Document]) -> SearchOutcome {
    let terms = query.terms();
    if terms.is_empty() || query.limit == 0 {
        return SearchOutcome {
            results: Vec::new(),
            total_matches: 0,
        };
    }

    let folded_query = fold_case(query.text.trim());
    let mut candidates = documents
        .iter()
        .filter_map(|document| candidate_for_document(document, &folded_query, &terms))
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .result
            .score
            .cmp(&left.result.score)
            .then_with(|| left.result.title.cmp(&right.result.title))
            .then_with(|| left.sort_path.cmp(&right.sort_path))
    });

    let total_matches = candidates.len();
    let results = candidates
        .into_iter()
        .take(query.limit)
        .map(|candidate| candidate.result)
        .collect();

    SearchOutcome {
        results,
        total_matches,
    }
}

fn candidate_for_document(
    document: &Document,
    folded_query: &str,
    terms: &[String],
) -> Option<Candidate> {
    let mut matches = Vec::with_capacity(terms.len());
    for term in terms {
        matches.push(best_match_for_term(document, term)?);
    }

    let best_field = matches
        .iter()
        .min_by_key(|term_match| term_match.field)
        .map(|term_match| term_match.field)
        .unwrap_or(SearchMatchedField::Content);
    let snippet = matches
        .iter()
        .find(|term_match| term_match.snippet.is_some())
        .and_then(|term_match| term_match.snippet.clone());

    let mut score = matches
        .iter()
        .map(|term_match| term_match.score)
        .sum::<u32>();
    score += full_query_bonus(document, folded_query);
    score += terms.len() as u32;

    Some(Candidate {
        result: SearchResult {
            document_path: document.path.clone(),
            title: document.title.clone(),
            relative_path: document.relative_path.clone(),
            score,
            matched_field: best_field,
            snippet,
        },
        sort_path: fold_case(&document.relative_path.to_string_lossy()),
    })
}

fn best_match_for_term(document: &Document, term: &str) -> Option<TermMatch> {
    let mut best: Option<TermMatch> = None;

    consider(
        &mut best,
        match_text(&document.title, term, SearchMatchedField::Title, 8_000),
    );
    consider(
        &mut best,
        match_text(
            &document.file_name.to_string_lossy(),
            term,
            SearchMatchedField::FileName,
            6_000,
        ),
    );
    consider(
        &mut best,
        match_text(
            &document.relative_path.to_string_lossy(),
            term,
            SearchMatchedField::RelativePath,
            5_000,
        ),
    );

    for (name, value) in &document.properties {
        consider(
            &mut best,
            match_text(name, term, SearchMatchedField::Frontmatter, 4_000),
        );
        for value in string_property_values(value) {
            consider(
                &mut best,
                match_text(value, term, SearchMatchedField::Frontmatter, 3_500),
            );
        }
    }

    if let Some(byte_index) = find_case_insensitive(&document.markdown_content, term) {
        consider(
            &mut best,
            Some(TermMatch {
                score: 2_000,
                field: SearchMatchedField::Content,
                snippet: Some(snippet_around(&document.markdown_content, byte_index, term)),
            }),
        );
    }

    best
}

fn full_query_bonus(document: &Document, folded_query: &str) -> u32 {
    if folded_query.is_empty() {
        return 0;
    }

    let title = fold_case(&document.title);
    if title == folded_query {
        20_000
    } else if title.starts_with(folded_query) {
        12_000
    } else if title.contains(folded_query) {
        8_000
    } else if fold_case(&document.file_name.to_string_lossy()).contains(folded_query) {
        4_000
    } else if fold_case(&document.relative_path.to_string_lossy()).contains(folded_query) {
        3_000
    } else {
        0
    }
}

fn match_text(
    value: &str,
    term: &str,
    field: SearchMatchedField,
    base_score: u32,
) -> Option<TermMatch> {
    let folded = fold_case(value);
    if folded == term {
        Some(TermMatch {
            score: base_score + 1_000,
            field,
            snippet: None,
        })
    } else if folded.starts_with(term) {
        Some(TermMatch {
            score: base_score + 600,
            field,
            snippet: None,
        })
    } else if folded.contains(term) {
        Some(TermMatch {
            score: base_score,
            field,
            snippet: None,
        })
    } else {
        None
    }
}

fn consider(best: &mut Option<TermMatch>, candidate: Option<TermMatch>) {
    let Some(candidate) = candidate else {
        return;
    };

    match best {
        Some(best_match)
            if best_match
                .score
                .cmp(&candidate.score)
                .then_with(|| candidate.field.cmp(&best_match.field))
                .is_gt() => {}
        Some(_) => *best = Some(candidate),
        None => *best = Some(candidate),
    }
}

fn string_property_values(value: &PropertyValue) -> Vec<&str> {
    match value {
        PropertyValue::String(value) => vec![value.as_str()],
        PropertyValue::Array(values) => values
            .iter()
            .flat_map(string_property_values)
            .collect::<Vec<_>>(),
        PropertyValue::Object(values) => values
            .values()
            .flat_map(string_property_values)
            .collect::<Vec<_>>(),
        PropertyValue::Null | PropertyValue::Bool(_) | PropertyValue::Number(_) => Vec::new(),
    }
}

fn find_case_insensitive(value: &str, folded_term: &str) -> Option<usize> {
    if folded_term.is_empty() {
        return None;
    }

    let (folded, byte_map) = fold_case_with_byte_map(value);
    let folded_index = folded.find(folded_term)?;
    byte_map.get(folded_index).copied()
}

fn fold_case_with_byte_map(value: &str) -> (String, Vec<usize>) {
    let mut folded = String::with_capacity(value.len());
    let mut byte_map = Vec::with_capacity(value.len());

    for (original_byte_index, character) in value.char_indices() {
        for folded_character in character.to_lowercase() {
            let mut buffer = [0; 4];
            let encoded = folded_character.encode_utf8(&mut buffer);
            folded.push_str(encoded);
            byte_map.extend(std::iter::repeat_n(original_byte_index, encoded.len()));
        }
    }

    (folded, byte_map)
}

fn snippet_around(value: &str, byte_index: usize, folded_term: &str) -> String {
    const CONTEXT: usize = 42;
    let start = char_boundary_before(value, byte_index, CONTEXT);
    let end = char_boundary_after(value, byte_index, folded_term.chars().count() + CONTEXT);
    let mut snippet = value[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if start > 0 {
        snippet.insert_str(0, "...");
    }
    if end < value.len() {
        snippet.push_str("...");
    }

    snippet
}

fn char_boundary_before(value: &str, byte_index: usize, max_chars: usize) -> usize {
    value[..byte_index]
        .char_indices()
        .rev()
        .nth(max_chars)
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn char_boundary_after(value: &str, byte_index: usize, max_chars: usize) -> usize {
    value[byte_index..]
        .char_indices()
        .nth(max_chars)
        .map(|(index, _)| byte_index + index)
        .unwrap_or(value.len())
}

fn fold_case(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use super::{search_documents, SearchMatchedField, SearchQuery, DEFAULT_SEARCH_LIMIT};
    use crate::{Document, DocumentMetadata, PropertyValue};

    use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

    #[test]
    fn finds_exact_title_match() {
        let documents = vec![document("carf.md", "CARF", [], "")];

        let results = search_documents(SearchQuery::new("CARF"), &documents);

        assert_eq!(titles(&results.results), vec!["CARF"]);
        assert_eq!(results.results[0].matched_field, SearchMatchedField::Title);
    }

    #[test]
    fn finds_title_prefix() {
        let documents = vec![document("carf.md", "Reforma do CARF", [], "")];

        let results = search_documents(SearchQuery::new("Reforma"), &documents);

        assert_eq!(titles(&results.results), vec!["Reforma do CARF"]);
    }

    #[test]
    fn finds_title_contains() {
        let documents = vec![document("carf.md", "Notas sobre CARF", [], "")];

        let results = search_documents(SearchQuery::new("CARF"), &documents);

        assert_eq!(titles(&results.results), vec!["Notas sobre CARF"]);
    }

    #[test]
    fn finds_file_name() {
        let documents = vec![document("projects/carf-report.md", "Relatório", [], "")];

        let results = search_documents(SearchQuery::new("carf-report"), &documents);

        assert_eq!(titles(&results.results), vec!["Relatório"]);
        assert_eq!(
            results.results[0].matched_field,
            SearchMatchedField::FileName
        );
    }

    #[test]
    fn finds_relative_path() {
        let documents = vec![document("meetings/reforma-carf.md", "Ata", [], "")];

        let results = search_documents(SearchQuery::new("meetings"), &documents);

        assert_eq!(titles(&results.results), vec!["Ata"]);
        assert_eq!(
            results.results[0].matched_field,
            SearchMatchedField::RelativePath
        );
    }

    #[test]
    fn finds_frontmatter_key() {
        let documents = vec![document(
            "doc.md",
            "Doc",
            [("responsavel", string("Sergio"))],
            "",
        )];

        let results = search_documents(SearchQuery::new("responsavel"), &documents);

        assert_eq!(titles(&results.results), vec!["Doc"]);
        assert_eq!(
            results.results[0].matched_field,
            SearchMatchedField::Frontmatter
        );
    }

    #[test]
    fn finds_frontmatter_string_value() {
        let documents = vec![document("doc.md", "Doc", [("owner", string("Sergio"))], "")];

        let results = search_documents(SearchQuery::new("sergio"), &documents);

        assert_eq!(titles(&results.results), vec!["Doc"]);
    }

    #[test]
    fn finds_markdown_content() {
        let documents = vec![document(
            "doc.md",
            "Doc",
            [],
            "Responsável pelo julgamento de recursos fiscais.",
        )];

        let results = search_documents(SearchQuery::new("fiscais"), &documents);

        assert_eq!(titles(&results.results), vec!["Doc"]);
        assert_eq!(
            results.results[0].matched_field,
            SearchMatchedField::Content
        );
    }

    #[test]
    fn search_is_case_insensitive() {
        let documents = vec![document("carf.md", "CARF", [], "")];

        assert_eq!(
            titles(&search_documents(SearchQuery::new("carf"), &documents).results),
            vec!["CARF"]
        );
        assert_eq!(
            titles(&search_documents(SearchQuery::new("Carf"), &documents).results),
            vec!["CARF"]
        );
    }

    #[test]
    fn search_handles_unicode_case() {
        let documents = vec![document("acoes/visao.md", "Visão Tributária", [], "")];

        let results = search_documents(SearchQuery::new("visão"), &documents);

        assert_eq!(titles(&results.results), vec!["Visão Tributária"]);
    }

    #[test]
    fn ranking_is_deterministic() {
        let documents = vec![
            document("b.md", "Nota CARF", [], ""),
            document("a.md", "Nota CARF", [], ""),
        ];

        let results = search_documents(SearchQuery::new("carf"), &documents);

        assert_eq!(
            paths(&results.results),
            vec![PathBuf::from("a.md"), PathBuf::from("b.md")]
        );
    }

    #[test]
    fn exact_title_ranks_above_content_match() {
        let documents = vec![
            document("content.md", "Documento", [], "CARF"),
            document("title.md", "CARF", [], ""),
        ];

        let results = search_documents(SearchQuery::new("CARF"), &documents);

        assert_eq!(titles(&results.results), vec!["CARF", "Documento"]);
    }

    #[test]
    fn multiple_terms_require_all_terms_and_score_them() {
        let documents = vec![
            document("one.md", "CARF Fiscal", [], ""),
            document("two.md", "CARF", [], ""),
            document("three.md", "Outro", [], "carf fiscal administrativo"),
        ];

        let results = search_documents(SearchQuery::new("carf fiscal"), &documents);

        assert_eq!(titles(&results.results), vec!["CARF Fiscal", "Outro"]);
    }

    #[test]
    fn content_match_includes_short_snippet() {
        let documents = vec![document(
            "doc.md",
            "Doc",
            [],
            "Órgão colegiado responsável pelo julgamento de recursos fiscais no Brasil.",
        )];

        let results = search_documents(SearchQuery::new("julgamento"), &documents);

        let snippet = results.results[0].snippet.as_deref().unwrap();
        assert!(snippet.contains("julgamento"));
        assert!(snippet.len() < 120);
    }

    #[test]
    fn no_result_is_empty() {
        let documents = vec![document("doc.md", "Doc", [], "")];

        let results = search_documents(SearchQuery::new("xyz"), &documents);

        assert!(results.results.is_empty());
        assert_eq!(results.total_matches, 0);
    }

    #[test]
    fn empty_query_is_empty() {
        let documents = vec![document("doc.md", "Doc", [], "carf")];

        let results = search_documents(SearchQuery::new("  "), &documents);

        assert!(results.results.is_empty());
    }

    #[test]
    fn result_limit_is_applied() {
        let documents = (0..60)
            .map(|index| document(&format!("doc-{index}.md"), &format!("CARF {index}"), [], ""))
            .collect::<Vec<_>>();

        let results = search_documents(
            SearchQuery::with_limit("CARF", DEFAULT_SEARCH_LIMIT),
            &documents,
        );

        assert_eq!(results.results.len(), DEFAULT_SEARCH_LIMIT);
        assert_eq!(results.total_matches, 60);
    }

    #[test]
    fn removed_document_disappears_from_next_search() {
        let mut documents = vec![
            document("one.md", "CARF", [], ""),
            document("two.md", "Fiscal", [], ""),
        ];
        documents.retain(|document| document.title != "CARF");

        let results = search_documents(SearchQuery::new("CARF"), &documents);

        assert!(results.results.is_empty());
    }

    #[test]
    fn modified_document_changes_search_results() {
        let mut documents = vec![document("doc.md", "Doc", [], "tributário")];
        documents[0].markdown_content = String::from("fiscal");

        assert!(search_documents(SearchQuery::new("tributário"), &documents)
            .results
            .is_empty());
        assert_eq!(
            titles(&search_documents(SearchQuery::new("fiscal"), &documents).results),
            vec!["Doc"]
        );
    }

    #[test]
    fn created_document_enters_next_search() {
        let mut documents = vec![document("one.md", "One", [], "")];
        documents.push(document("two.md", "Fiscal", [], ""));

        let results = search_documents(SearchQuery::new("Fiscal"), &documents);

        assert_eq!(titles(&results.results), vec!["Fiscal"]);
    }

    #[test]
    fn handles_one_thousand_documents() {
        let documents = (0..1_000)
            .map(|index| {
                document(
                    &format!("docs/doc-{index}.md"),
                    &format!("Doc {index}"),
                    [],
                    "carf",
                )
            })
            .collect::<Vec<_>>();

        let results = search_documents(SearchQuery::new("carf"), &documents);

        assert_eq!(results.results.len(), DEFAULT_SEARCH_LIMIT);
        assert_eq!(results.total_matches, 1_000);
    }

    fn document<const N: usize>(
        relative_path: &str,
        title: &str,
        properties: [(&str, PropertyValue); N],
        markdown_content: &str,
    ) -> Document {
        let relative_path = PathBuf::from(relative_path);
        let path = PathBuf::from("/workspace").join(&relative_path);
        Document {
            path,
            relative_path: relative_path.clone(),
            file_name: relative_path
                .file_name()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| OsString::from("document.md")),
            metadata: DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.to_owned(),
            markdown_content: markdown_content.to_owned(),
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect::<BTreeMap<_, _>>(),
            document_type: None,
            collection_id: String::from("documents"),
            warnings: Vec::new(),
        }
    }

    fn titles(results: &[super::SearchResult]) -> Vec<&str> {
        results
            .iter()
            .map(|result| result.title.as_str())
            .collect::<Vec<_>>()
    }

    fn paths(results: &[super::SearchResult]) -> Vec<PathBuf> {
        results
            .iter()
            .map(|result| result.relative_path.clone())
            .collect::<Vec<_>>()
    }

    fn string(value: &str) -> PropertyValue {
        PropertyValue::String(value.to_owned())
    }
}
