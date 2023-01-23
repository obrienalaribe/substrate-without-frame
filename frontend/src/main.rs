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

fn take_input() -> (String, u32) {
	let mut input = String::new();
	println!("Please Enter: [toAddress Amount]");

	io::stdin()
		.read_line(&mut input)
		.expect("Failed to read line");

	let input: String = input.trim().parse().unwrap();

	let payload: Vec<&str> = input.split(" ").collect();

	if payload.len() == 2 {
		let amount: u32 = payload[1].parse().unwrap();
		return (payload[0].to_string(), amount)
	}else{
		process::exit(1);
	}
}

fn fees(transaction: Transaction) -> Transaction {
	let mut transaction = transaction.clone();
	let mut input = String::new();
	println!("Enter Fee: [Amount]");

	io::stdin()
		.read_line(&mut input)
		.expect("Failed to read line");

	let fee: u32 = input.trim().parse().expect("Fee needs to be a u32");
	transaction.fee = fee;
	transaction.send_amount = transaction.send_amount;
	return transaction;
}

#[tokio::main]
async fn main(){
	let p1_mnemonic = "grass rib slab system month zoo observe render display shallow clay venture";
	let p2_mnemonic = "scan true window staff cloth loud accuse retreat tent rack glimpse genuine";

	let keypair = sr25519::Pair::from_string(p1_mnemonic, None).unwrap();
	let mint_keypair_ext: BasicExtrinsic = BasicExtrinsic::new_unsigned(Call::Mint(keypair.public(), 0));
	send_extrinsic(mint_keypair_ext).await;
	println!("");
	println!("################################");
	println!("Your Account: {}", keypair.to_owned().public());

	let keypair_p2 = sr25519::Pair::from_string(p2_mnemonic, None).unwrap();
	let mint_keypair2_ext: BasicExtrinsic = BasicExtrinsic::new_unsigned(Call::Mint(keypair_p2.public(), 0));
	send_extrinsic(mint_keypair2_ext).await;
	println!("John's Account: {}", keypair_p2.to_owned().public());
	println!("################################");

	let mut address_map: HashMap<String, Public> = HashMap::new();
	address_map.insert(format!("{}", keypair.to_owned().public()), keypair.public());
	address_map.insert(format!("{}", keypair_p2.to_owned().public()), keypair_p2.public());

	loop {
		println!("Please input an action: [mint, transfer, balance]");

		let mut action = String::new();

		io::stdin()
			.read_line(&mut action)
			.expect("Failed to read line");

		let command: String = action.trim().parse().expect("Please type an action!");

		match command.as_str() {
			c if c.to_lowercase().contains("mint") => {
				let (address, amount) = take_input();
				let public_key:&Public  = address_map.get(&address).unwrap();
				let mint_extrinsic: BasicExtrinsic = BasicExtrinsic::new_unsigned(Call::Mint(*public_key, amount));
				send_extrinsic(mint_extrinsic).await;
				println!("Minted {:?} to {:?}",amount, address );
			},
			c if c.to_lowercase().contains("transfer") => {
				let (address, amount) = take_input();
				let user_pub_key: Public  = *address_map.get(&format!("{}", keypair.to_owned().public())).unwrap();
				let recipient_pub_key: Public  = *address_map.get(&address).unwrap();

				let raw_transaction = Transaction {
					from: user_pub_key,
					to: recipient_pub_key,
					send_amount: amount,
					fee: 0
				};

				println!("Raw Transaction {:?}", raw_transaction);

				let transaction = fees(raw_transaction);

				let message_hash = blake2_256(&*transaction.clone().encode());
				let signature = keypair.sign(&message_hash);

				let transfer_extrinsic: BasicExtrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));
				let response = send_extrinsic(transfer_extrinsic).await;

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
