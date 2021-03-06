use super::*;
use select::{
    document::Document,
    predicate::Name,
};

#[derive(Debug)]
pub struct Domain {
    pub base: String,
    pub indexables: Vec<String>,
    pub host: String,
}

const PROTOCOL: &str = "https://";

impl Domain {
    /// parse the input arg, create url request string and new struct - can unwrap host, because &input_arg has already been parsed
    pub fn new(input_arg: String) -> Result<Self, RError> {
        let domain = Url::parse(&input_arg)?;
        let origin = domain.host().unwrap();
        let mut u = String::from(PROTOCOL);
        u.push_str(&origin.to_string());
        Ok(Self {
            base: u,
            indexables: Vec::new(),
            host: origin.to_string(),
        })
    }

    /// request initial doc from domain, process href tags, return HashSet to ensure unique values
    pub async fn process_domain_links(&mut self) -> Result<(), RError> {
        let origin = self.base.clone();
        let formed = check_protocol(origin);
        let res = reqwest::get(formed)
            .await?
            .text()
            .await?;
        let links: HashSet<String> = Document::from(res.as_str())
            .find(Name("a"))
            .filter_map(|n| n.attr("href"))
            .map(|n| n.to_owned())
            .collect::<HashSet<String>>();
        self.indexables = parse_links(self.base.clone(), links);
        Ok(())
    }
}

/// loop links from domain, if link is path only, append domain, if link base is substring add as indexable
fn parse_links(base: String, links: HashSet<String>) -> Vec<String> {
    let mut indexables = Vec::new();
    for link in links {
        if link.starts_with('/') {
            let full_u = format!("{}{}", base, &link);
            indexables.push(full_u);
        }
        if link.contains(&base) {
            indexables.push(link)
        }
    }
    indexables
}

/// append protocol on origin for http client
fn check_protocol(org: String) -> String {
    if !org.contains(PROTOCOL) {
        let mut u = String::from(PROTOCOL);
        u.push_str(&org.to_string());
        return u;
    }
    org
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn test_domain() {
        let arg_str = String::from("https://blog.com/blog/async-tests-tokio-rust/");
        let d = Domain::new(arg_str);
        assert!(d.is_ok());

        let arg_str = String::from("blog.com/----/");
        let d = Domain::new(arg_str);
        assert!(d.is_err());
    }

    #[test]
    fn test_check_protocol() {
        let arg_tes1 = String::from("https://blog.com");

        assert_eq!(check_protocol(String::from("https://blog.com")), arg_tes1);
        assert_eq!(check_protocol(String::from("blog.com")), arg_tes1);
    }

    #[test]
    fn test_parse_links() {
        let mut set = HashSet::new();
        set.insert("example-base.com".to_owned());
        set.insert("/path/path".to_owned());
        set.insert("/example/path".to_owned());
        set.insert("https://example-base.com".to_owned());
        let results = parse_links("example-base.com".to_owned(), set);
        let filtered = results
            .into_iter()
            .map(|s|  check_protocol(s))
            .filter(|it| {
                let r = Regex::new(r"^(https)://+example-base.com+([a-zA-Z0-9\\/]*)$");
                r.unwrap().is_match(it)
            }).collect::<Vec<String>>();

        assert_eq!(filtered.len(), 4);
    }
}
