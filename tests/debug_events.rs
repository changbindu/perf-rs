use perf_rs::core::perf_data::{PerfDataReader, Event};

#[test]
fn debug_simple_events() {
    let path = ".sisyphus/evidence/reference-simple.perf.data";
    let mut reader = PerfDataReader::from_path(path).expect("Failed to open");
    
    println!("Data offset: {}, size: {}", reader.data_offset(), reader.data_size());
    
    let iter = reader.event_iter().expect("Failed to create iter");
    let mut count = 0;
    
    for result in iter {
        match result {
            Ok(event) => {
                count += 1;
                println!("Event {}: {:?}", count, event);
                if count >= 10 {
                    break;
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}
