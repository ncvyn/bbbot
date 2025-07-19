use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::BufReader;
use tokio::sync::Mutex;

pub async fn parse_xml(xml_feed: &Mutex<String>) {
    let response = reqwest::get(xml_feed.lock().await.as_str())
        .await
        .expect("Request failed");
    let bytes = response
        .bytes()
        .await
        .expect("Failed to read response bytes");
    let mut reader = Reader::from_reader(BufReader::new(bytes.as_ref()));

    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut count = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let name = reader
                    .decoder()
                    .decode(name.as_ref())
                    .expect("Failed to decode name");
                println!("read start event {:?}", name.as_ref());
                count += 1;
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (),
        }
    }

    println!("{count} start tags found.");
}
