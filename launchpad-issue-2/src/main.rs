use anyhow::Result;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::{response::IntoResponse, routing::get, Router};
use std::fmt::Display;
use std::{io::BufReader, sync::Arc};
use xml::reader::XmlEvent;
use xml::EventReader;

#[derive(Debug)]
struct Podcast {
    title: String,
    description: String,
    audio_file: Option<String>,
}

impl Podcast {
    fn new() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            audio_file: None,
        }
    }

    fn to_html(&self) -> String {
        format!(
            r#"
            <html>
                <head>
                    <title>My Podcast: {}</title>
                </head>
                <body>
                    <h1>{}</h1>
                    <p>{}</p>
                    <audio controls src="{}"></audio>
                </body>
            </html>
        "#,
            self.title,
            self.title,
            self.description,
            match self.audio_file {
                Some(ref file) => file,
                None => "No audio available",
            }
        )
    }
}

impl Display for Podcast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"
            <html>
                <head>
                    <title>My Podcast: {}</title>
                </head>
                <body>
                    <h1>{}</h1>
                    <p>{}</p>
                    <audio controls src="{}"></audio>
                </body>
            </html>
        "#,
            self.title,
            self.title,
            self.description,
            match self.audio_file {
                Some(ref file) => file,
                None => "No audio available",
            }
        )
    }
}

enum ParseState {
    Start,
    InTitle,
    InDescription,
}

type AppState = Arc<Vec<Podcast>>;

async fn root(State(app_state): State<AppState>) -> impl IntoResponse {
    let response = format!(
        r#"
<html>
    <head>
        <title>My Podcasts</title>
    </head>
    <body>
        <h1>My Podcasts</h1>
        <ul>
            {}
        </ul>
    </body>
</html>
    "#,
        app_state
            .iter()
            .enumerate()
            .map(|(id, podcast)| { format!(r#"<li><a href="/{}">{}</a></li>"#, id, podcast.title) })
            .collect::<Vec<String>>()
            .join("\n")
    );
    Html(response)
}

async fn podcast(State(app_state): State<AppState>, Path(id): Path<usize>) -> impl IntoResponse {
    let podcast = app_state.get(id);
    Html(match podcast {
        Some(podcast) => podcast.to_html(),
        None => "No podcast found".to_string(),
    })
}

async fn read_podcasts_from_xml(url: &str) -> Result<Vec<Podcast>> {
    let mut results = Vec::new();
    let data = reqwest::get(url).await?.text().await?;
    let parser = EventReader::new(BufReader::new(data.as_bytes()));
    let mut podcast = Podcast::new();
    let mut state = ParseState::Start;
    for event in parser {
        match event {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "title" => state = ParseState::InTitle,
                "description" => state = ParseState::InDescription,
                "enclosure" => {
                    podcast.audio_file = attributes.into_iter().find_map(|attr| {
                        if attr.name.local_name == "url" {
                            Some(attr.value)
                        } else {
                            None
                        }
                    });
                }
                _ => {}
            },
            Ok(XmlEvent::CData(content)) => match state {
                ParseState::InTitle => {
                    podcast.title = content;
                    state = ParseState::Start;
                }
                ParseState::InDescription => {
                    podcast.description = content;
                    state = ParseState::Start;
                }
                _ => {}
            },
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "item" {
                    results.push(podcast);
                    podcast = Podcast::new();
                    state = ParseState::Start;
                }
            }
            _ => {}
        }
    }
    Ok(results)
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let podcasts = read_podcasts_from_xml("https://workingdraft.de/feed/").await?;
    let app_state = Arc::new(podcasts);
    let router = Router::new()
        .route("/", get(root))
        .route("/:id", get(podcast))
        .with_state(app_state);

    Ok(router.into())
}
