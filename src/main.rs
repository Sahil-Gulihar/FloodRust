use std::io::{self, Write, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Default)]
struct PingStats {
    packets_transmitted: u64,
    packets_received: u64,
    bytes_transferred: u64,
    time_ms: u64,
}

fn main() {
    print!("Enter the Link without 'https://' : ");
    io::stdout().flush().unwrap();
    let mut ipv6_address = String::new();
    io::stdin()
        .read_line(&mut ipv6_address)
        .expect("Failed to read line");
    let ipv6_address = ipv6_address.trim().to_string();

    print!("Enter the duration in seconds: ");
    io::stdout().flush().unwrap();
    let mut duration = String::new();
    io::stdin()
        .read_line(&mut duration)
        .expect("Failed to read line");
    let duration: u64 = duration
        .trim()
        .parse()
        .expect("Please enter a valid number");
   
    let thread_count = num_cpus::get();
    println!("Using {} threads", thread_count);

    let should_stop = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];
    let stats = Arc::new(Mutex::new(vec![PingStats::default(); thread_count]));

    for i in 0..thread_count {
        let ipv6 = ipv6_address.clone();
        let stop_flag = Arc::clone(&should_stop);
        let thread_stats = Arc::clone(&stats);
        
        let handle = thread::spawn(move || {
            println!("Thread {} starting", i);
            let mut child = Command::new("ping")
                .arg("-6")
                .arg("-f")
                .arg("-i")
                .arg("0.002")
                .arg("-s")
                .arg("1024")
                .arg(&ipv6)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to execute command");

            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);

            let start_time = Instant::now();

            // Read stdout
            let stdout_stop_flag = Arc::clone(&stop_flag);
            let stdout_stats = Arc::clone(&thread_stats);
            thread::spawn(move || {
                println!("Thread {} stdout reader starting", i);
                for line in stdout_reader.lines() {
                    if let Ok(line) = line {
                        println!("Thread {} received: {}", i, line);
                        let dot_count = line.chars().filter(|&c| c == '.').count() as u64;
                        if dot_count > 0 {
                            let mut stats = stdout_stats.lock().unwrap();
                            stats[i].packets_received += dot_count;
                            stats[i].bytes_transferred += dot_count * 1024; // Each dot represents a 1024-byte packet
                            println!("Thread {} updated stats: received={}, bytes={}", 
                                     i, stats[i].packets_received, stats[i].bytes_transferred);
                        }
                    }
                    if stdout_stop_flag.load(Ordering::Relaxed) {
                        break;
                    }
                }
                println!("Thread {} stdout reader finished", i);
            });

            // Read stderr
            let stderr_stop_flag = Arc::clone(&stop_flag);
            thread::spawn(move || {
                println!("Thread {} stderr reader starting", i);
                for line in stderr_reader.lines() {
                    if let Ok(line) = line {
                        eprintln!("Thread {} Error: {}", i, line);
                    }
                    if stderr_stop_flag.load(Ordering::Relaxed) {
                        break;
                    }
                }
                println!("Thread {} stderr reader finished", i);
            });

            while !stop_flag.load(Ordering::Relaxed) && start_time.elapsed() < Duration::from_secs(duration) {
                thread::sleep(Duration::from_millis(100));
            }

            child.kill().expect("Failed to kill the process");
            child.wait().expect("Failed to wait on child");

            let mut stats = thread_stats.lock().unwrap();
            stats[i].packets_transmitted = stats[i].packets_received; // In flood ping, transmitted = received
            stats[i].time_ms = start_time.elapsed().as_millis() as u64;

            println!("Thread {} finished. Final stats: transmitted={}, received={}, bytes={}", 
                     i, stats[i].packets_transmitted, stats[i].packets_received, stats[i].bytes_transferred);
        });

        handles.push(handle);
    }

    thread::sleep(Duration::from_secs(duration));
    should_stop.store(true, Ordering::Relaxed);

    for handle in handles {
        handle.join().unwrap();
    }

    println!("All threads finished.");

    let final_stats = stats.lock().unwrap();
    let mut total_stats = PingStats::default();

    for (i, stat) in final_stats.iter().enumerate() {
        println!("Thread {} statistics:", i);
        let (packets_lost, loss_percentage) = if stat.packets_transmitted >= stat.packets_received {
            (stat.packets_transmitted - stat.packets_received,
             (stat.packets_transmitted - stat.packets_received) as f64 / stat.packets_transmitted as f64 * 100.0)
        } else {
            (0, 0.0)
        };
        
        println!("  Packets: Transmitted = {}, Received = {}, Lost = {} ({:.2}% loss)",
                 stat.packets_transmitted,
                 stat.packets_received,
                 packets_lost,
                 loss_percentage);
        println!("  Bytes transferred: {} ({:.2} MB)",
                 stat.bytes_transferred,
                 stat.bytes_transferred as f64 / 1_000_000.0);
        println!("  Time: {} ms", stat.time_ms);
        println!("  Bandwidth: {:.2} Mbps", 
                 if stat.time_ms > 0 {
                     (stat.bytes_transferred * 8) as f64 / stat.time_ms as f64 / 1000.0
                 } else {
                     0.0
                 });

        total_stats.packets_transmitted += stat.packets_transmitted;
        total_stats.packets_received += stat.packets_received;
        total_stats.bytes_transferred += stat.bytes_transferred;
        total_stats.time_ms = total_stats.time_ms.max(stat.time_ms);
    }

    println!("\nTotal statistics:");
    let (total_packets_lost, total_loss_percentage) = if total_stats.packets_transmitted >= total_stats.packets_received {
        (total_stats.packets_transmitted - total_stats.packets_received,
         (total_stats.packets_transmitted - total_stats.packets_received) as f64 / total_stats.packets_transmitted as f64 * 100.0)
    } else {
        (0, 0.0)
    };
    
    println!("  Packets: Transmitted = {}, Received = {}, Lost = {} ({:.2}% loss)",
             total_stats.packets_transmitted,
             total_stats.packets_received,
             total_packets_lost,
             total_loss_percentage);
    println!("  Bytes transferred: {} ({:.2} MB)",
             total_stats.bytes_transferred,
             total_stats.bytes_transferred as f64 / 1_000_000.0);
    println!("  Time: {} ms", total_stats.time_ms);
    println!("  Total Bandwidth: {:.2} Mbps", 
             if total_stats.time_ms > 0 {
                 (total_stats.bytes_transferred * 8) as f64 / total_stats.time_ms as f64 / 1000.0
             } else {
                 0.0
             });
}