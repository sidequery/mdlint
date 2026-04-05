use crate::config::Config;
use crate::file::MarkdownFile;
use crate::headings::Heading;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub files: Vec<MarkdownFile>,
    /// Lowercased basename (without extension) -> list of file indices
    pub basename_index: HashMap<String, Vec<usize>>,
    /// File path -> list of headings (for anchor resolution)
    pub heading_index: HashMap<PathBuf, Vec<Heading>>,
    /// Alias -> file index
    pub alias_index: HashMap<String, Vec<usize>>,
    /// All known file paths (absolute) for O(1) existence checks
    pub known_paths: HashSet<PathBuf>,
    /// Lowercased absolute path string -> actual path, for case-insensitive resolution
    pub path_case_index: HashMap<String, PathBuf>,
}

impl Workspace {
    pub fn from_directory(
        dir: &Path,
        config: &Config,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let dir = dir.canonicalize()?;
        let (md_paths, all_paths) = discover_files(&dir, config)?;
        Self::build(dir, md_paths, all_paths, config)
    }

    pub fn from_files(
        files: &[PathBuf],
        config: &Config,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let root = std::env::current_dir()?;
        let all_paths: Vec<PathBuf> = files
            .iter()
            .map(|p| {
                if p.is_absolute() {
                    p.clone()
                } else {
                    root.join(p)
                }
            })
            .collect();
        let md_paths = all_paths
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .cloned()
            .collect();
        Self::build(root, md_paths, all_paths, config)
    }

    fn build(
        root: PathBuf,
        md_paths: Vec<PathBuf>,
        all_paths: Vec<PathBuf>,
        config: &Config,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let slug_mode = config.links.slug_mode;

        // Parse markdown files in parallel
        let files: Vec<MarkdownFile> = md_paths
            .par_iter()
            .filter_map(|path| match MarkdownFile::parse(path, &root, slug_mode) {
                Ok(f) => Some(f),
                Err(e) => {
                    eprintln!("warning: could not read {}: {}", path.display(), e);
                    None
                }
            })
            .collect();

        // Build path indexes from ALL discovered files (not just .md)
        let mut known_paths = HashSet::with_capacity(all_paths.len());
        let mut path_case_index = HashMap::with_capacity(all_paths.len());
        for path in &all_paths {
            known_paths.insert(path.clone());
            let key = path.to_string_lossy().to_lowercase();
            path_case_index.insert(key, path.clone());
        }

        // Build markdown-specific indexes
        let mut basename_index: HashMap<String, Vec<usize>> = HashMap::new();
        let mut heading_index: HashMap<PathBuf, Vec<Heading>> = HashMap::new();
        let mut alias_index: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, file) in files.iter().enumerate() {
            let basename = file
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            basename_index.entry(basename).or_default().push(i);

            heading_index.insert(file.path.clone(), file.headings.clone());

            if let Some(ref fm) = file.frontmatter {
                for alias in fm.aliases() {
                    alias_index
                        .entry(alias.to_lowercase())
                        .or_default()
                        .push(i);
                }
            }
        }

        Ok(Workspace {
            root,
            files,
            basename_index,
            heading_index,
            alias_index,
            known_paths,
            path_case_index,
        })
    }

    pub fn file_by_path(&self, path: &Path) -> Option<&MarkdownFile> {
        self.files.iter().find(|f| f.path == path)
    }
}

/// Returns (md_paths, all_paths) - markdown files and all files for path indexing.
fn discover_files(
    dir: &Path,
    config: &Config,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn std::error::Error + Send + Sync>> {
    use ignore::overrides::OverrideBuilder;

    let mut builder = ignore::WalkBuilder::new(dir);
    builder.hidden(true);
    builder.git_ignore(true);
    builder.git_global(true);

    let mut overrides = OverrideBuilder::new(dir);
    for pattern in &config.workspace.exclude {
        overrides.add(&format!("!{pattern}"))?;
    }
    builder.overrides(overrides.build()?);

    let mut md_paths = Vec::new();
    let mut all_paths = Vec::new();
    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            all_paths.push(path.to_path_buf());
            if path.extension().is_some_and(|e| e == "md") {
                md_paths.push(path.to_path_buf());
            }
        }
    }

    Ok((md_paths, all_paths))
}
