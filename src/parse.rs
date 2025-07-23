use chrono::{DateTime, Utc};
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::json::json;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "_id")]
    id: String,
    last_updated: i64,
    due_assignments: Vec<Assignment>,
}

#[derive(Deserialize)]
struct Assignment {
    subject: String,
    title: String,
    due_date: String,
}

pub async fn parse_xml(secrets: &str, client: Client) -> Vec<CreateEmbed> {
    let [xml_feed, restdb_api_key, restdb_database]: [&str; 3] = secrets
        .split_ascii_whitespace()
        .collect::<Vec<&str>>()
        .try_into()
        .expect("Failed to parse secrets");

    let restdb_response = client
        .get(format!("{restdb_database}/rest/data"))
        .header("content-type", "application/json")
        .header("x-apikey", restdb_api_key)
        .send()
        .await
        .expect("Failed to fetch RESTDB data");

    let database = restdb_response
        .json::<Vec<Entry>>()
        .await
        .expect("Failed to parse JSON response");

    let entry = database
        .first()
        .expect("No entries found in RESTDB response");

    let last_updated =
        DateTime::<Utc>::from_timestamp(entry.last_updated, 0).expect("Invalid timestamp");
    let id: &str = &entry.id;

    let response = client
        .get(xml_feed)
        .send()
        .await
        .expect("Failed to fetch XML feed");
    let bytes = response
        .bytes()
        .await
        .expect("Failed to read response bytes");

    let mut reader = Reader::from_str(std::str::from_utf8(&bytes).expect("Invalid UTF-8"));
    reader.config_mut().trim_text(true);

    let mut embeds: Vec<CreateEmbed> = Vec::new();
    let mut curr_date: i64 = 0;
    let mut ignore_content = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"published" => {
                let text = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value");

                let date = DateTime::parse_from_rfc3339(&text).expect("Invalid date format");

                if date < last_updated {
                    curr_date = Utc::now().timestamp();
                    break;
                }
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"title" => {
                let text = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value");

                match text.to_lowercase() {
                    ref x if x.contains("submission received") => ignore_content = true,
                    ref x if x.contains("new content") => ignore_content = true,
                    ref x if x.contains("new items") => ignore_content = true,
                    ref x if x.contains("due soon") => ignore_content = true,
                    _ => {}
                }
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"content" => {
                if ignore_content {
                    ignore_content = false;
                    continue;
                }

                let text = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");

                let md = html2md::rewrite_html(&text, false);
                let md = md.lines().filter(|c| *c != "|").collect::<Vec<&str>>();

                println!("{md:?}");

                let embed = CreateEmbed::new()
                    .title(format!("{} [{}]", md[1], md[0]))
                    .description(format!("**{}:** {}", md[2], md[3]));

                embeds.push(embed);
            }

            Ok(Event::Eof) => break,

            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),

            _ => (),
        }
    }

    client
        .put(format!("{restdb_database}/rest/data/{id}"))
        .header("content-type", "application/json")
        .header("x-apikey", restdb_api_key)
        .json(&json!(
            {
                "last_updated": curr_date,
                "due_assignments": [
                    {
                        "subject": "Example subject",
                        "title": "Example title",
                        "due_date": "Example due date",
                    }
                ],
            }
        ))
        .send()
        .await
        .expect("Failed to send data to RESTDB");

    embeds
}
