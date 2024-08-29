// Spencer (Spinjrock) Oswald, 8/28/24
// Sources:
// https://wiki.vg/RCON
// https://mctools.readthedocs.io/en/master/rcon.html

use std::io::prelude::*;
use std::io;
use std::net::TcpStream;
use std::error::Error;
use std::{thread, time};

const RCON_TARGET: &str = "127.0.0.1:25575";
const RCON_PASSWORD: &str = "debug";
const UID: u32 = 100;
const LOGIN: u32 = 3;
const COMMAND: u32 = 2;
const MULTI_PACKET: u32 = 0;
const RESP_DELAY: time::Duration = time::Duration::from_millis(500);
const SERVER_MAX_MESSAGE_SIZE: usize = 4110;
const CLIENT_MAX_MESSAGE_SIZE: u32 = 1460;


#[derive(Debug)]
struct Message {
    length: u32,                // Length of the remainder of the packet (length field itself not included)
    request_id: u32,            // Client Generated ID
    packet_type: u32,           // Type of packet: 3 - login, 2 - command, 0 - multi-packet response
    payload: Vec<u8>,           // Null terminated string
}

impl Message {
    fn new(request_id: u32, packet_type: u32, payload: Vec<u8>) -> Message {
        let length = u32::try_from(10 + payload.len()).unwrap();
        if length > CLIENT_MAX_MESSAGE_SIZE {
            panic!("Message exceeds maximum client message size");
        }
        let ret = Message {
            length: length,
            request_id: request_id,
            packet_type: packet_type,
            payload: payload.clone(),
        };
        return ret;
    }
    fn serialize(&self) -> Vec<u8> {
        let mut ret: Vec<u8> = Vec::new();
        for b in self.length.to_le_bytes() {
            ret.push(b);
        }
        for b in self.request_id.to_le_bytes() {
            ret.push(b);
        }
        for b in self.packet_type.to_le_bytes() {
            ret.push(b);
        }
        for b in self.payload.clone() {
            ret.push(b);
        }
        ret.push(0x00);          // Null Terminator
        ret.push(0x00);          // Random required padding byte
        return ret;
    }
    fn from_deserialize(message: &[u8]) -> Result<Message, Box<dyn Error>> {
        let length = u32::from_le_bytes(message[0..4].try_into()?);
        let payload_offset: usize = (length + 3).try_into().unwrap();       //The last index of the payload should be 4 (for the length field) - 1 (for the padding byte) + length
        let request_id = u32::from_le_bytes(message[4..8].try_into()?);
        let packet_type = u32::from_le_bytes(message[8..12].try_into()?);
        let payload: Vec<u8> = message[12..payload_offset].to_vec();

        Ok(Message {
            length: length,
            request_id: request_id,
            packet_type: packet_type,
            payload: payload,
        })

    }
}

fn login(mut stream: &TcpStream) {
    let login = Message::new(
        UID,
        LOGIN,
        RCON_PASSWORD.as_bytes().to_vec()
    );
    let response = send_message(&login, &mut stream);
    if response.request_id != UID {
        panic!("Failed to login to {}, are you sure the rcon password is correct?", stream.peer_addr().unwrap())
    }
    
}

fn send_command(command: &str, mut stream: &TcpStream) -> String {
    let command = Message::new(
        UID,
        COMMAND,
        command.as_bytes().to_vec()
    );
    let response = send_message(&command, &mut stream);
    let parsed_response = String::from_utf8(response.payload);
    match parsed_response {
        Ok(s) => s,
        Err(e) => panic!("Failed to parse utf-8 from message: {}", e)
    }
}

fn send_message(message: &Message, mut stream: &TcpStream) -> Message {
    let serial = message.serialize();
    match stream.write(&serial) {
        Ok(_) => (),
        Err(e) => panic!("Failed to send message to {}, Error: {}", stream.peer_addr().unwrap(), e)
    }
    thread::sleep(RESP_DELAY);    // A hacky way to dodge the delay in msgs
    let mut response = [0xff as u8; SERVER_MAX_MESSAGE_SIZE];
    match stream.read(&mut response) {
        Ok(_) => (),
        Err(e) => panic!("Failed to read response from {}, Error: {}", stream.peer_addr().unwrap(), e)
    }
    match Message::from_deserialize(&response) {
        Ok(message) => message,
        Err(e) => panic!("Failed to parse message from server {}, Error: {}", stream.peer_addr().unwrap(), e)
    }


}

fn main() {
    std::io::stdout().flush()
        .expect("Failed to flust stdout");

    let mut stream = TcpStream::connect(RCON_TARGET)
        .expect("Failed to connect to target");
    println!("Connected! {}", stream.peer_addr().unwrap());
    
    println!("Logging in...");
    login(&mut stream);
    println!("Logged in!");
    
    let res = send_command("/list", &stream);
    println!("{res}");
    
    loop {
        let mut command = String::new();
        print!("rcon> ");
        io::stdout().flush().expect("Failed to flush stdout");
        io::stdin().read_line(&mut command)
            .expect("Failed to parse user input");
        let command = command.trim();
        if command == "exit" {
            break;
        }
        let response = send_command(&command, &stream);
        println!("{response}");
    }

}
