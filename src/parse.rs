use chrono::{DateTime, Utc};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use tokio::sync::Mutex;

const CUTOFF_DAYS: i64 = 7;

pub async fn parse_xml(xml_feed: &Mutex<String>) {
    let response = reqwest::get(xml_feed.lock().await.as_str())
        .await
        .expect("Request failed");
    let bytes = response
        .bytes()
        .await
        .expect("Failed to read response bytes");

    let mut reader = Reader::from_str(std::str::from_utf8(&bytes).expect("Invalid UTF-8"));

    reader.config_mut().trim_text(true);

    let mut count = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"published" => {
                let text = reader
                    .read_text(e.name())
                    .expect("Cannot decode text value");

                let date = DateTime::parse_from_rfc3339(&text).expect("Invalid date format");
                let cutoff = Utc::now() - chrono::Duration::days(CUTOFF_DAYS);

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
                        println!("New announcement found: {}", text);
                    }
                    _ => {}
                }
                count += 1;
            }

            Ok(Event::Eof) => break,

            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),

            _ => (),
        }
    }

    println!("{count} titles found.");
}
