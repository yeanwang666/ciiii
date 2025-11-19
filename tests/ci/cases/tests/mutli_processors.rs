use std::thread;
use std::time::Instant;

const PROCESSORS: u8 = 4;

#[test]
fn mutli_processors_smoke() {
    // Size of the computation
    let n: u64 = 500_000_000;

    println!("Running single-threaded computation...");
    let t1 = Instant::now();
    let single_result = compute_range_sum(0, n);
    let single_time = t1.elapsed();
    println!(
        "Single-thread result = {single_result}, time = {:?}",
        single_time
    );

    println!("\nRunning multi-threaded computation...");
    println!("Using {PROCESSORS} threads");

    let chunk = n / PROCESSORS as u64;

    let t2 = Instant::now();

    // Spawn worker threads
    let mut handles = Vec::new();
    for i in 0..PROCESSORS {
        let start = i as u64 * chunk;
        let end = if i == PROCESSORS - 1 {
            n
        } else {
            start + chunk
        };

        handles.push(thread::spawn(move || compute_range_sum(start, end)));
    }

    // Collect results
    let mut multi_result = 0u64;
    for h in handles {
        multi_result = multi_result.wrapping_add(h.join().unwrap());
    }

    let multi_time = t2.elapsed();
    println!(
        "Multi-thread result = {multi_result}, time = {:?}",
        multi_time
    );

    // === Validate ===
    assert_eq!(single_result, multi_result, "Results do not match!");
    assert!(
        multi_time < single_time,
        "Multithreaded computation is not faster!"
    );
}

fn compute_range_sum(start: u64, end: u64) -> u64 {
    (start..end).fold(0u64, |acc, x| acc.wrapping_add(x))
}
