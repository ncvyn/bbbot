use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub fn parse_xml() {
    let xml = r#"<tag1 att1 = "test">
                <tag2><!--Test comment-->Test</tag2>
                <tag2>Test 2</tag2>
             </tag1>"#;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut count = 0;
    let mut txt = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),

            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"tag1" => println!(
                    "attributes values: {:?}",
                    e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>()
                ),

                b"tag2" => {
                    count += 1;
                    // clippy don't kill me
                }

                _ => (),
            },

            Ok(Event::Text(e)) => txt.push(e.into_owned()),

            _ => (),
        }

        buf.clear();
    }

    println!("Count: {}", count);
    println!("Text: {:?}", txt);
}
