use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct PingStats {
    transmitted: u64,
    received: u64,
    bytes_transferred: u64,
    time_ms: u64,
}

fn get_ping_args() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["-c", "10000", "-i", "0.002", "-s", "1024"]
    } else {
        vec!["-6", "-f", "-i", "0.002", "-s", "1024"]
    }
}

fn main() {
    print!("Enter the URL: ");
    io::stdout().flush().unwrap();
    let mut link_address = String::new();
    io::stdin()
        .read_line(&mut link_address)
        .expect("Failed to read line");
    let link_address = link_address.trim().to_string();

    print!("Enter duration in seconds: ");
    io::stdout().flush().unwrap();
    let mut duration = String::new();
    io::stdin()
        .read_line(&mut duration)
        .expect("Failed to read line");
    let duration: u64 = duration.trim().parse().expect("Enter a valid number");

    let thread_count = num_cpus::get();
    println!("Using {} threads", thread_count);

    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(Mutex::new(vec![PingStats::default(); thread_count]));
    let mut handles = vec![];

    for i in 0..thread_count {
        let link = link_address.clone();
        let stop_flag = Arc::clone(&stop);
        let thread_stats = Arc::clone(&stats);

        let handle = thread::spawn(move || {
            println!("Thread {} starting", i);
            let mut child = Command::new("ping")
                .args(get_ping_args())
                .arg(&link)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to execute ping command");

            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            let read_output = BufReader::new(stdout);
            let read_error = BufReader::new(stderr);

            let start_time = Instant::now();

            let output_stop = Arc::clone(&stop_flag);
            let output_stats = Arc::clone(&thread_stats);

            let stdout_handle = thread::spawn(move || {
                println!("Thread {} stdout reader starting", i);
                for line in read_output.lines() {
                    if let Ok(line) = line {
                        println!("Thread {} received {}", i, line);
                        let dot_count = line.chars().filter(|&c| c == '.').count() as u64;
                        if dot_count > 0 {
                            let mut stats = output_stats.lock().unwrap();
                            stats[i].received += dot_count;
                            stats[i].bytes_transferred += dot_count * 1024;
                            println!(
                                "Thread {} updated stats: received={}, bytes={}",
                                i, stats[i].received, stats[i].bytes_transferred
                            );
                        }
                        if output_stop.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
                println!("Thread {} stdout reader finished", i);
            });

            let stderr_stop = Arc::clone(&stop_flag);
            let stderr_handle = thread::spawn(move || {
                println!("Thread {} stderr reader starting", i);
                for line in read_error.lines() {
                    if let Ok(line) = line {
                        eprintln!("Thread {} Error: {}", i, line);
                    }
                    if stderr_stop.load(Ordering::Relaxed) {
                        break;
                    }
                }
                println!("Thread {} stderr reader finished", i);
            });

            while !stop_flag.load(Ordering::Relaxed)
                && start_time.elapsed() < Duration::from_secs(duration)
            {
                thread::sleep(Duration::from_millis(100));
            }

            child.kill().expect("Failed to kill the process");
            child.wait().expect("Failed to wait on child");

            stdout_handle.join().expect("Failed to join stdout thread");
            stderr_handle.join().expect("Failed to join stderr thread");

            let mut stats = thread_stats.lock().unwrap();
            stats[i].transmitted = stats[i].received;
            stats[i].time_ms = start_time.elapsed().as_millis() as u64;

            println!(
                "Thread {} finished. Final stats: transmitted={}, received={}, bytes={}",
                i, stats[i].transmitted, stats[i].received, stats[i].bytes_transferred
            );
        });

        handles.push(handle);
    }

    thread::sleep(Duration::from_secs(duration));
    stop.store(true, Ordering::Relaxed);

    for handle in handles {
        handle.join().unwrap();
    }

    println!("All threads finished.");

    let final_stats = stats.lock().unwrap();
    let mut total = PingStats::default();

    for (i, stat) in final_stats.iter().enumerate() {
        println!("Thread {} statistics:", i);

        let (packets_lost, loss_percentage) = if stat.transmitted >= stat.received {
            (
                stat.transmitted - stat.received,
                (stat.transmitted - stat.received) as f64 / stat.transmitted as f64 * 100.0,
            )
        } else {
            (0, 0.0)
        };

        println!(
            "Packets: Transmitted = {}, Received = {}, Lost = {} ({:.2}% loss)",
            stat.transmitted, stat.received, packets_lost, loss_percentage
        );
        println!(
            "  Bytes transferred: {} ({:.2} MB)",
            stat.bytes_transferred,
            stat.bytes_transferred as f64 / 1_000_000.0
        );
        println!("  Time: {} ms", stat.time_ms);
        println!(
            "  Bandwidth: {:.2} Mbps",
            if stat.time_ms > 0 {
                (stat.bytes_transferred * 8) as f64 / stat.time_ms as f64 / 1000.0
            } else {
                0.0
            }
        );

        total.transmitted += stat.transmitted;
        total.received += stat.received;
        total.bytes_transferred += stat.bytes_transferred;
        total.time_ms = total.time_ms.max(stat.time_ms);
    }

    println!("\nTotal statistics:");
    let (total_packets_lost, total_loss_percentage) = if total.transmitted >= total.received {
        (
            total.transmitted - total.received,
            (total.transmitted - total.received) as f64 / total.transmitted as f64 * 100.0,
        )
    } else {
        (0, 0.0)
    };
    println!(
        "  Packets: Transmitted = {}, Received = {}, Lost = {} ({:.2}% loss)",
        total.transmitted, total.received, total_packets_lost, total_loss_percentage
    );
    println!(
        "  Bytes transferred: {} ({:.2} MB)",
        total.bytes_transferred,
        total.bytes_transferred as f64 / 1_000_000.0
    );
    println!("  Time: {} ms", total.time_ms);
    println!(
        "  Total Bandwidth: {:.2} Mbps",
        if total.time_ms > 0 {
            (total.bytes_transferred * 8) as f64 / total.time_ms as f64 / 1000.0
        } else {
            0.0
        }
    );
}
