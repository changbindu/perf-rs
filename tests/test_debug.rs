use perf_rs::core::perf_data::{PerfDataReader, Event};

fn main() {
    let path = ".sisyphus/evidence/reference-simple.perf.data";
    match PerfDataReader::from_path(path) {
        Ok(mut reader) => {
            println!("Header: {:?}", reader.header());
            println!("Attrs: {:?}", reader.attrs());
            println!("Data offset: {}, size: {}", reader.data_offset(), reader.data_size());

            let events = reader.read_all_events();
            match events {
                Ok(ev) => {
                    println!("Total events read: {}", ev.len());
                    for (i, e) in ev.iter().take(10).enumerate() {
                        println!("Event {}: {:?}", i, e);
                    }
                }
                Err(e) => println!("Error reading events: {}", e),
            }
        }
        Err(e) => println!("Error opening file: {}", e),
    }
}
