# DDOS Utility Written in Rust

This utility allows you to send multiple requests to any URL using a flood ping technique, implemented in Rust.

## Installation and Usage

Follow these steps to install and use the DDOS Utility:

1. Clone the repository:
    ```bash
    git clone https://github.com/sahil-gulihar/floodrust
    ```

2. Navigate to the project directory:
    ```bash
    cd FloodRust
    ```

3. Build the project using Cargo:
    ```bash
    cargo build
    ```

4. Run the utility:
    ```bash
    cargo run
    ```

## Concept

### Flood Ping
A ping flood is a type of Denial-of-Service (DoS) attack where an attacker overwhelms a target system with an excessive number of ICMP echo requests (pings). This can cause the target system to become unresponsive to legitimate traffic.

### Multithreading
This utility leverages Rust's multithreading capabilities to perform more efficient DDOS attacks, allowing multiple threads to send requests simultaneously, thus increasing the attack's effectiveness.

### Bypassing Rate Limiting
To bypass rate limiting mechanisms of websites, a 0.02 ms delay is added between requests. This helps to avoid detection and ensures the requests are sent continuously without interruption.

### IPv6
The utility supports IPv6, the latest version of the IP protocol. IPv6 provides more IP addresses, improved security features, and simplified network configuration compared to IPv4.


This tool is intended for educational purposes only. Use it at your own risk. Misuse of this utility can lead to legal consequences. Always ensure you have permission before performing any testing or attacks on any network or system.
