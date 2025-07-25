use serde_bencode::value;
use serde_json;
use std::{collections::HashMap, env};

// Usage: your_program.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    let list = 'l';
    let dict = 'd';
    //string have 4:home
    //numbers have i-3e
    //list [a,b] = l1:a1:be
    //dictionary {'cow': 'moo', 'spam': 'eggs'} = d3:cow3:moo4:spam4:eggse 
    if command == "decode" {
        eprintln!("Logs from your program will appear here!");
        let encoded_value = &args[2];
        match &encoded_value.chars().next() {
            Some(value) => {
                if value.is_digit(10){
                let decoded_value : String = serde_bencode::from_str(&encoded_value).unwrap();
                println!("{}", decoded_value);
                }
                else if value.eq(&list) {
                    let decoded_value : Vec<String> = serde_bencode::from_str(&encoded_value).unwrap();
                    println!("{:?}", decoded_value);
                } 
                // else if value.eq(&dict) {
                //     let decoded_value : HashMap<> = serde_bencode::from_str(&encoded_value).unwrap();
                //     println!("{:?}", decoded_value);
                // }
                else{
                    let decoded_value : i64 = serde_bencode::from_str(&encoded_value).unwrap();
                    println!("{}", decoded_value);
                }
            },
            _ => panic!("Cannot decode: empty input string")
        } 
    } else {
        println!("unknown command: {}", args[1])
    }
}
