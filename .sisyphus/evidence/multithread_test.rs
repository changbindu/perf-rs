use std::thread;
use std::time::Duration;

fn main() {
    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                for _ in 0..1000 {
                    let mut x = 0u64;
                    for j in 0..100 {
                        x += j as u64;
                    }
                    thread::sleep(Duration::from_micros(10));
                }
                println!("Thread {} completed", i);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("All threads completed");
}
