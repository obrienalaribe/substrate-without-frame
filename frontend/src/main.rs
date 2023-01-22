use std::collections::HashMap;
use std::{io, process};
use runtime::*;
use serde::{Deserialize, Serialize};
use parity_scale_codec::{Encode, Decode};
use reqwest::header::CONTENT_TYPE;
use sp_core::hexdisplay::HexDisplay;
use sp_core::sr25519::{Public, Signature};
use sp_core::{blake2_256, Pair, sr25519};
use sp_runtime::traits::Extrinsic;

#[derive(Serialize, Debug, Deserialize)]
struct JSONRPCResponse {
	jsonrpc: String,
	result: Option<String>,
	id: u32,
}

async fn send_extrinsic(extrinsic: BasicExtrinsic) -> JSONRPCResponse {
	let encoding = extrinsic.encode();

	// println!("Encoding: {:?}", encoding);
	let hex_payload = HexDisplay::from(&encoding);

	let body = format!(r#"
			{{
				"jsonrpc":"2.0",
				"id":1,
				"method":"author_submitExtrinsic",
				"params": ["{}"]
			}}"#, hex_payload);

	let client = reqwest::Client::new();
	let resp_json: JSONRPCResponse = client.post(" http://localhost:9933")
		.body(body.clone())
		.header(CONTENT_TYPE, "application/json")
		.send()
		.await
		.unwrap()
		.json::<JSONRPCResponse>()
		.await
		.unwrap();

	// println!("{:?}", body.clone());
	resp_json
}

async fn get_state_request(public_key: Public) -> u32 {
	let encoding = Encode::encode(&public_key);
	// println!("State Request Encoding: {:?}", encoding);
	let hex_payload = HexDisplay::from(&encoding);

	let body = format!(r#"
	{{
		"jsonrpc":"2.0",
		"id":1,
		"method":"state_getStorage",
		"params": ["{}"]
	}}"#, hex_payload);

	let client = reqwest::Client::new();
	let resp_json: JSONRPCResponse = client.post(" http://localhost:9933")
		.body(body.clone())
		.header(CONTENT_TYPE, "application/json")
		.send()
		.await
		.unwrap()
		.json::<JSONRPCResponse>()
		.await
		.unwrap();

	let mut response = hex::decode(resp_json.result.unwrap().split_at(2).1).unwrap();
	let balance = u32::decode(&mut &response[..]).unwrap();
	balance
}

fn take_input() -> (String, String) {
	let mut input = String::new();
	println!("Please Enter: [toAddress Amount]");

	io::stdin()
		.read_line(&mut input)
		.expect("Failed to read line");

	let input: String = input.trim().parse().unwrap();

	let payload: Vec<&str> = input.split(" ").collect();

	if payload.len() == 2 {
		println!("{} {}", payload[0], payload[1]);
		return (payload[0].to_string(), payload[1].to_string())
	}else{
		process::exit(1);
	}

}

#[tokio::main]
async fn main(){
	// Generate 2 accounts from seed
	let p1_mnemonic = "grass rib slab system month zoo observe render display shallow clay venture";
	let p2_mnemonic = "scan true window staff cloth loud accuse retreat tent rack glimpse genuine";

	let keypair = sr25519::Pair::from_string(p1_mnemonic, None).unwrap();
	println!("Your Public Key: {}", keypair.to_owned().public());
	let keypair_p2 = sr25519::Pair::from_string(p2_mnemonic, None).unwrap();
	println!("John's Public Key: {}", keypair_p2.to_owned().public());

	let mut address_map: HashMap<String, Public> = HashMap::new();
	address_map.insert(format!("{}", keypair.to_owned().public()), keypair.public());
	address_map.insert(format!("{}", keypair_p2.to_owned().public()), keypair_p2.public());
	println!("Map{:?}", address_map);

	loop {
		println!("Please input an action: [mint, transfer, balance]");

		let mut action = String::new();

		io::stdin()
			.read_line(&mut action)
			.expect("Failed to read line");

		let command: String = action.trim().parse().expect("Please type an action!");

		match command.as_str() {
			c if c.contains("mint") => {
				let (address, amount) = take_input();
				let public_key:&Public  = address_map.get(&address).unwrap();
				let mint_extrinsic: BasicExtrinsic = BasicExtrinsic::new_unsigned(Call::Mint(*public_key, 100));
				send_extrinsic(mint_extrinsic).await;
				println!("Minted {:?} to {:?}",amount, address );
			},
			c if c.to_lowercase().contains("transfer") => {
				let (address, amount) = take_input();
				let user_pub_key: Public  = *address_map.get(&format!("{}", keypair.to_owned().public())).unwrap();
				let recipient_pub_key: Public  = *address_map.get(&address).unwrap();

				let transaction = Transaction {
					from: user_pub_key,
					to: recipient_pub_key,
					send_amount: 5
				};

				let message_hash = blake2_256(&*transaction.clone().encode());
				let signature = keypair.sign(&message_hash);

				// let verify_signature = sr25519::Pair::verify(&signature, message_hash, &keypair.public());
				// println!("Signature Verification Result: {:?}", verify_signature);

				let transfer_extrinsic: BasicExtrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));
				let response = send_extrinsic(transfer_extrinsic).await;
				println!("{:?}", response);

			},
			c if c.to_lowercase().contains("balance") => {
				let mut input = String::new();
				println!("Please Enter: [Address]");

				io::stdin()
					.read_line(&mut input)
					.expect("Failed to read line");

				let input: String = input.trim().parse().unwrap();

				let payload: Vec<&str> = input.split(" ").collect();

				if payload.len() == 1 {
					let address = payload[0].to_string();
					let public_key:&Public  = address_map.get(&address).unwrap();
					let balance = get_state_request(*public_key).await;
					println!("Account balance is: {:?}", balance);
				}

			},
			_ => { println!("Invalid Command, exiting ..."); process::exit(1);}
		}

	}

}
