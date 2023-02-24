#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use html2md::parse_html;
use readability::extractor::extract;
use reqwest::Client;
use spider::{url::Url, website::Website};
use std::{
    fs::{self, DirEntry},
    io::Cursor,
    path::{Path, PathBuf},
    process,
};
use tracing::debug;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use ::clippy::{
    build_prompt, into_document, openai::ModelType, search_project, Document, OpenAI, Qdrant,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Embed { slug: String },
    Process { slug: String },
    Ask { slug: String, query: String },
    Fetch { slug: String, repo: String },
    Query { slug: String, query: String },
    Crawl { slug: String, base_url: String },
}

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() {
    dotenv().ok();
    let cli = Cli::parse();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "cli=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    match cli.command {
        Commands::Fetch { slug, repo } => {
            if fs::metadata(format!("build/{slug}")).is_ok() {
                eprintln!("Error: Directory already exists");
                process::exit(1);
            }

            fs::create_dir_all(format!("build/{slug}")).expect("Failed to create directory");

            let client = Client::new();
            let repo_url = format!("https://github.com/{repo}/archive/refs/heads/main.zip");

            let archive = client
                .get(&repo_url)
                .send()
                .await
                .expect("Failed to fetch repository")
                .error_for_status()
                .unwrap()
                .bytes()
                .await
                .unwrap();

            zip_extract::extract(
                Cursor::new(archive),
                &PathBuf::from(format!("build/{slug}")),
                true,
            )
            .expect("Failed to extract zip file");

            preprocess_archive(format!("build/{slug}")).unwrap();

            let qdrant = Qdrant::new();
            qdrant
                .create_collection(&format!("docs_{slug}"))
                .await
                .unwrap();
        }
        Commands::Crawl { slug, base_url } => {
            if fs::metadata(format!("build/{slug}")).is_ok() {
                eprintln!("Error: Directory already exists");
                process::exit(1);
            }

            fs::create_dir_all(format!("build/{slug}")).expect("Failed to create directory");

            let mut website = Website::new(&base_url);
            website.configuration.user_agent = "Clippy".into();
            website.on_link_find_callback = |url| {
                debug!("Found link: {url}");

                url
            };

            website.scrape().await;

            for page in website.get_pages() {
                if !page.get_url().starts_with(&base_url) {
                    debug!("Skipping unrelated page: {}", page.get_url());
                    continue;
                }

                let url = Url::parse(page.get_url()).unwrap();
                let path = url.path();
                let doc = extract(&mut page.get_html().as_bytes(), &url).unwrap();
                let mut markdown = parse_html(&doc.content);

                if !doc.title.is_empty() {
                    markdown = format!("---\ntitle: \"{}\"\n---\n{}", doc.title, markdown);
                }

                if markdown.is_empty() {
                    debug!("Skipping empty page: {url}");
                    continue;
                }

                let file_path = PathBuf::from(format!(
                    "build/{slug}{path}{}.md",
                    if path.ends_with('/') { "index" } else { "" }
                ));

                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(file_path, markdown).unwrap();
            }

            preprocess_archive(format!("build/{slug}")).unwrap();

            let qdrant = Qdrant::new();
            qdrant
                .create_collection(&format!("docs_{slug}"))
                .await
                .unwrap();
        }
        Commands::Process { slug } => {
            if fs::metadata(format!("build/{slug}")).is_err() {
                eprintln!("Error: Project does not exist");
                process::exit(1);
            }

            let files = read_dir_recursive(format!("build/{slug}")).unwrap();
            for file in files {
                let document = into_document(&file, format!("build/{slug}")).unwrap();

                if !document.sections.is_empty() {
                    fs::write(
                        file.path().with_extension("json"),
                        serde_json::to_string_pretty(&document).unwrap(),
                    )
                    .unwrap();
                }

                fs::remove_file(file.path()).unwrap();
            }
        }
        Commands::Embed { slug } => {
            if fs::metadata(format!("build/{slug}")).is_err() {
                eprintln!("Error: Project does not exist");
                process::exit(1);
            }

            let client = OpenAI::new();
            let qdrant = Qdrant::new().collection(&format!("docs_{slug}"));
            let files = read_dir_recursive(format!("build/{slug}")).unwrap();

            for file in files {
                let document = fs::read_to_string(file.path()).unwrap();
                let document: Document = serde_json::from_str(&document).unwrap();

                let points = client.embed(&document).await.unwrap();

                qdrant.upsert(&points).await.unwrap();
            }
        }
        Commands::Query { slug, query } => {
            if fs::metadata(format!("build/{slug}")).is_err() {
                eprintln!("Error: Project does not exist");
                process::exit(1);
            }

            let results = search_project(&format!("docs_{slug}"), &query)
                .await
                .unwrap();

            println!("{results:?}");
        }
        Commands::Ask { slug, query } => {
            let client = OpenAI::new();
            let qdrant = Qdrant::new().collection(&format!("docs_{slug}"));

            let query_points = client.raw_embed(&query).await.unwrap();
            let results = qdrant.query(query_points).await.unwrap();
            let response = client
                .prompt(&build_prompt(&query, &results), ModelType::Davinci)
                .await
                .unwrap();

            println!("{response:?}");
        }
    }
}

fn preprocess_archive<P: AsRef<Path>>(path: P) -> Result<()> {
    let extensions = vec!["md", "mdx"];

    map_dir(path, &|file| {
        if file
            .path()
            .extension()
            .map_or(true, |ext| !extensions.contains(&ext.to_str().unwrap()))
        {
            fs::remove_file(file.path())?;
        }

        Ok(())
    })
}

fn read_dir_recursive<P: AsRef<Path>>(path: P) -> Result<Vec<DirEntry>> {
    let files = fs::read_dir(path)?.collect::<Result<Vec<_>, std::io::Error>>()?;

    Ok(files
        .into_iter()
        .flat_map(|entry| {
            if entry.path().is_dir() {
                read_dir_recursive(entry.path())
            } else {
                Ok(vec![entry])
            }
        })
        .flatten()
        .collect())
}

fn map_dir<P: AsRef<Path>>(path: P, cb: &impl Fn(DirEntry) -> Result<()>) -> Result<()> {
    let files = read_dir_recursive(path)?;

    for file in files {
        cb(file)?;
    }

    Ok(())
}
