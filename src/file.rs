use crate::frontmatter::{self, Frontmatter};
use crate::headings::{self, Heading, SlugMode};
use crate::links::{self, Link};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct MarkdownFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub frontmatter: Option<Frontmatter>,
}

impl MarkdownFile {
    pub fn parse(path: &Path, root_dir: &Path, slug_mode: SlugMode) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let relative_path = path
            .strip_prefix(root_dir)
            .unwrap_or(path)
            .to_path_buf();

        let arena = Arena::new();
        let options = comrak_options();
        let doc = parse_document(&arena, &content, &options);

        let headings = headings::extract_headings(doc, slug_mode);
        let links = links::extract_links(doc, &content);
        let frontmatter = extract_frontmatter(doc);

        Ok(MarkdownFile {
            path: path.to_path_buf(),
            relative_path,
            content,
            headings,
            links,
            frontmatter,
        })
    }
}

fn comrak_options() -> Options<'static> {
    let mut opts = Options::default();
    opts.extension.wikilinks_title_after_pipe = true;
    opts.extension.front_matter_delimiter = Some("---".to_string());
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.footnotes = true;
    opts.parse.smart = false;
    opts
}

fn extract_frontmatter<'a>(root: &'a AstNode<'a>) -> Option<Frontmatter> {
    for child in root.children() {
        let data = child.data.borrow();
        if let NodeValue::FrontMatter(ref raw) = data.value {
            return frontmatter::parse_frontmatter(raw);
        }
    }
    None
}
