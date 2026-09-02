pub const AUTHOR_LINKEDIN_URL: &str = "https://www.linkedin.com/in/sergiocardososp/";
pub const AUTHOR_WEBSITE_URL: &str = "https://sergiocardoso.dev";
pub const AUTHOR_EMAIL: &str = "contato@sergiocardoso.org";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutContactLink {
    LinkedIn,
    Website,
    Email,
}

impl AboutContactLink {
    pub const fn target(self) -> &'static str {
        match self {
            Self::LinkedIn => AUTHOR_LINKEDIN_URL,
            Self::Website => AUTHOR_WEBSITE_URL,
            Self::Email => "mailto:contato@sergiocardoso.org",
        }
    }
}

pub fn open_about_contact(link: AboutContactLink) -> Result<(), String> {
    open::that(link.target()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{AboutContactLink, AUTHOR_EMAIL, AUTHOR_LINKEDIN_URL, AUTHOR_WEBSITE_URL};

    #[test]
    fn about_contact_urls_are_centralized_and_exact() {
        assert_eq!(AboutContactLink::LinkedIn.target(), AUTHOR_LINKEDIN_URL);
        assert_eq!(
            AUTHOR_LINKEDIN_URL,
            "https://www.linkedin.com/in/sergiocardososp/"
        );
        assert_eq!(AboutContactLink::Website.target(), AUTHOR_WEBSITE_URL);
        assert_eq!(AUTHOR_WEBSITE_URL, "https://sergiocardoso.dev");
        assert_eq!(AUTHOR_EMAIL, "contato@sergiocardoso.org");
    }

    #[test]
    fn about_email_link_uses_mailto() {
        assert_eq!(
            AboutContactLink::Email.target(),
            "mailto:contato@sergiocardoso.org"
        );
    }
}
