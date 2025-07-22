use chrono::{DateTime, Utc};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use reqwest::Client;

const DAYS_CUTOFF: i64 = 7;

pub async fn parse_xml(secrets: &str, client: Client) -> String {
    let [xml_feed, _restdb_api_key, _restdb_database]: [&str; 3] = secrets
        .split_ascii_whitespace()
        .collect::<Vec<&str>>()
        .try_into()
        .expect("Failed to parse secrets");

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

    let mut message = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"published" => {
                let text = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value");

                let date = DateTime::parse_from_rfc3339(&text).expect("Invalid date format");
                let cutoff = Utc::now() - chrono::Duration::days(DAYS_CUTOFF);

                if date < cutoff {
                    break;
                }
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"title" => {
                let text = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value");

                match text {
                    ref x if x.contains("New announcement") => {
                        message.push_str(&format!("**New Announcement:** {text}\n"));
                    }
                    _ => {}
                }
            }

            Ok(Event::Eof) => break,

            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),

            _ => (),
        }
    }

    message
}
