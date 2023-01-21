
use runtime::*;
use serde::{Deserialize, Serialize};
use parity_scale_codec::{Encode, Decode};
use reqwest::header::CONTENT_TYPE;
use sp_core::hexdisplay::HexDisplay;
use sp_core::sr25519::Signature;
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

	println!("Encoding: {:?}", encoding);
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

	println!("{:?}", body.clone());
	resp_json
}

#[tokio::main]
async fn main(){
	let transaction = Transaction {
		from: [1u8; 32],
		to: [2u8; 32],
		send_amount: 5000 //sending more than account balance => Err()
	};

	let keypair = sr25519::Pair::generate().0;
	let message_hash = blake2_256(&*transaction.clone().encode());
	let signature = keypair.sign(&message_hash);

	let verify_signature = sr25519::Pair::verify(&signature, message_hash, &keypair.public());
	println!("Signature Verification Result: {:?}", verify_signature);

	let transfer_extrinsic: BasicExtrinsic = <BasicExtrinsic as sp_runtime::traits::Extrinsic>::new(Call::Transfer(transaction), Some(signature)).unwrap();
	println!("{:?}", send_extrinsic(transfer_extrinsic).await);

	println!("{:?}", transaction);

}
