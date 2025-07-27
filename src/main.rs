#![allow(unused_imports)]
use serde::de;
use serde_bencode::value;
use serde_json::{self, Number, Value};
use std::{collections::HashMap, env};



fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    match &encoded_value.chars().next() {
        Some('i') => {
            if let Some((digit, rest)) = 
                encoded_value.
                    split_at(1)
                    .1
                    .split_once('e')
                    .and_then(|(digits,rest)|{
                        let n = digits.parse::<i64>().ok()?;
                        return Some((n, rest))
                    }){
                        return (digit.into(), rest);
                    }
        }
        Some('l') => {
            let mut res = Vec::new();
            let mut rem  = encoded_value.split_at(1).1;
            while let Some(c) = rem.chars().next() {
                if c == 'e' {
                    break;
                }
                let (result, rest) = decode_bencoded_value(rem);
                res.push(result);
                rem = rest;
            }
            return (res.into(), &rem[1..])

        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut rem = encoded_value.split_at(1).1;
            while !rem.is_empty() && !rem.starts_with('e'){
                let (key, rest) = decode_bencoded_value(rem);
                let (value, rest) = decode_bencoded_value(rest);
                if let serde_json::Value::String(key_string) = key{
                    dict.insert(key_string, value);
                }
                rem = rest;
            }
            return (dict.into(), &rem[1..])
        }
        Some('0'..='9') => {
            if let Some((len, rest)) = encoded_value.split_once(':'){
                if let Ok(len) = len.parse::<usize>() {
                    return (serde_json::Value::String(rest[..len].to_string()), &rest[len..]);  
                }
            }
        }
        _ => {}
    }    
    panic!("Unhandled encoded value: {}", encoded_value);
}

// Usage: your_program.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
   
    //string have 4:home
    //numbers have i-3e
    //list [a,b] = l1:a1:be
    //dictionary {'cow': 'moo', 'spam': 'eggs'} = d3:cow3:moo4:spam4:eggse 
    if command == "decode" {
        eprintln!("Logs from your program will appear here!");
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value).0;
        println!("{decoded_value}");
    } else {
        println!("unknown command: {}", args[1])
    }
}
