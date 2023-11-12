use std::fs;
use std::process::Command;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use reqwest;
use tokio::time::sleep;
use winreg::enums::*;
use winreg::RegKey;
//use libc::*;

#[derive(Deserialize)]
struct Config {
    server_address: String,
    server_port: u16,
    commands: Vec<CommandConfig>,
}

#[derive(Deserialize)]
struct CommandConfig {
    command: String,
    interval_seconds: u64,
}

#[derive(Serialize)]
struct CommandOutput {
    command: String,
    output: String,
    device_id: String
}

fn read_config() -> Config {
    let config_str = fs::read_to_string("config.json")
        .expect("Failed to read config.json");

    serde_json::from_str(&config_str)
        .expect("Failed to parse config.json")
}

fn get_device_id() -> String {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(r"SOFTWARE\MICROSOFT\Cryptography").unwrap();
    let device_id: String = key.get_value("MachineGuid").unwrap();
    device_id
}


fn execute_command(command: &str) -> String {
    let args: Vec<&str> = command.split_whitespace().collect();

    let output = Command::new(args[0])
        .args(&args[1..])
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).into_owned()
}

async fn send_output(server_address: &str, server_port: u16, command_output: &CommandOutput) {
    let json_payload = serde_json::to_string(command_output)
        .expect("Failed to serialize command output");

    let url = format!("http://{}:{}/api/commands", server_address, server_port);

    let client = reqwest::Client::new();
    let response = client.post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json_payload)
        .send()
        .await
        .expect("Failed to send post request");

    println!("Status: {}", response.status());
}

#[tokio::main]
async fn main() {
    let config = read_config();
    let device_uuid = get_device_id();

    for command in config.commands {
        let server_address = config.server_address.clone();
        let server_port = config.server_port;
        let command_interval = command.interval_seconds;
        let device_uuid = device_uuid.clone();

        tokio::spawn(async move {
            loop {
                let output_str = execute_command(&command.command);

                let command_output = CommandOutput {
                    command: command.command.clone(),
                    output: output_str,
                    device_id: device_uuid.clone(),
                };

                send_output(&server_address, server_port, &command_output).await;

                sleep(Duration::from_secs(command_interval)).await;
            }
        });
    }

    tokio::signal::ctrl_c().await.unwrap();
}
