mod heading;

use anyhow::Result;
use heading::Heading;
use map_macro::map;
use std::{
    collections::HashMap,
    fs::{self, DirEntry},
};
use yaml_front_matter::YamlFrontMatter;

#[derive(Debug, serde::Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub description: Option<String>,
}

pub fn parse_meta(content: &str) -> Result<(FrontMatter, String), Box<dyn std::error::Error>> {
    let document = YamlFrontMatter::parse::<FrontMatter>(content)?;

    Ok((document.metadata, document.content.trim().to_owned()))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MarkdownSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: String,
}

impl MarkdownSection {
    pub const fn default() -> Self {
        Self {
            title: None,
            content: String::new(),
        }
    }

    pub const fn with_title(title: Option<String>) -> Self {
        Self {
            title,
            content: String::new(),
        }
    }

    pub fn append(&mut self, line: &str) {
        if self.content.is_empty() {
            return self.content.push_str(line);
        }

        self.content.push_str(&format!("\n{line}"));
    }
}

struct State {
    current_section: usize,
    pub is_inside_code_block: bool,
    sections: Vec<MarkdownSection>,
    depth_map: HashMap<usize, String>,
}

impl State {
    pub fn with_title(title: String) -> Self {
        Self {
            current_section: 0,
            is_inside_code_block: false,
            sections: vec![MarkdownSection::default()],
            depth_map: map! {
                0 => title
            },
        }
    }

    pub fn toggle_code_block(&mut self) {
        self.is_inside_code_block = !self.is_inside_code_block;
    }

    pub fn compute_title(&mut self, heading: &Heading) -> String {
        self.depth_map
            .insert(heading.depth, heading.content.clone());

        let mut title = heading.content.clone();
        for (depth, sec_title) in &self.depth_map {
            if depth < &heading.depth {
                title = format!("{sec_title}: {title}");
            }
        }

        title
    }

    pub fn push_section(&mut self, section: MarkdownSection) {
        self.sections.push(section);
        self.current_section += 1;
    }

    pub fn push_line(&mut self, line: &str) {
        let curr_content = &self.sections[self.current_section].content;

        if !self.is_inside_code_block && curr_content.ends_with('\n') && curr_content.len() > 200 {
            self.push_section(MarkdownSection::with_title(
                self.sections[self.current_section].title.clone(),
            ));

            return self.push_line(line);
        }

        self.sections[self.current_section].append(line);
    }

    pub fn get_sections(self) -> Vec<MarkdownSection> {
        self.sections
            .into_iter()
            .filter(|section| !section.content.is_empty())
            .collect::<Vec<_>>()
    }
}

pub fn extract_sections(content: &str, metadata: &FrontMatter) -> Vec<MarkdownSection> {
    let mut state = State::with_title(metadata.title.clone());

    for line in content.lines() {
        if line.starts_with("```") {
            state.toggle_code_block();
        }

        if !state.is_inside_code_block {
            let heading = Heading::try_parse(line);

            if let Some(heading) = heading {
                let title = state.compute_title(&heading);
                state.push_section(MarkdownSection::with_title(Some(title)));

                continue;
            }
        }

        state.push_line(line);
    }

    state.get_sections()
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub path: String,
    pub title: String,
    description: Option<String>,
    pub sections: Vec<MarkdownSection>,
}

/// Parses a file into a document.
///
/// # Errors
/// - If the file cannot be read.
/// - If the file cannot be parsed.
/// - If the file path cannot be converted to a string.
/// - If the file path cannot be stripped from the base path.
pub fn into_document(file: &DirEntry, base_path: String) -> Result<Document> {
    let content = fs::read_to_string(file.path())?;

    let (metadata, content) = parse_meta(&content).map_err(|err| {
        anyhow::anyhow!(
            "Failed to parse front matter for file {}: {}",
            file.path().display(),
            err
        )
    })?;

    let sections = extract_sections(&content, &metadata);

    Ok(Document {
        sections,
        title: metadata.title,
        description: metadata.description,
        path: format!(
            "/{}",
            file.path()
                .strip_prefix(base_path)?
                .with_extension("")
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
        ),
    })
}
