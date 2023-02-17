#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
    fs::{self, DirEntry},
    io::Cursor,
    path::{Path, PathBuf},
    process,
};

use ::clippy::{
    build_prompt, into_document, openai::ModelType, search_project, Document, OpenAI, Qdrant,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use reqwest::Client;

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
    Fetch { slug: String, repo: String },
    Query { slug: String, query: String },
    Ask { slug: String, query: String },
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let cli = Cli::parse();

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
        Commands::Process { slug } => {
            if fs::metadata(format!("build/{slug}")).is_err() {
                eprintln!("Error: Project does not exist");
                process::exit(1);
            }

            let files = read_dir_recursive(format!("build/{slug}")).unwrap();
            for file in files {
                let document = into_document(&file, format!("build/{slug}")).unwrap();

                fs::write(
                    file.path().with_extension("json"),
                    serde_json::to_string_pretty(&document).unwrap(),
                )
                .unwrap();

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
