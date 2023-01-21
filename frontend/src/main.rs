use parity_scale_codec::{ Encode };
use sp_core::{sr25519, Pair};
use blake2::{Blake2s256};

pub type Address = [u8; 32];

#[derive(Encode)]
struct Transaction {
	from: Address,
	to: Address,
	amount: u128,
}

fn main() {
	let transaction = Transaction {
		from: [1u8; 32],
		to: [2u8; 32],
		amount: 5000
	};

	// let message = Hash(transaction);
	let signature = ();

	let encoding = transaction.encode();
	let decoded: Transaction =

	println!("{:?}", transaction.encode());
	println!("{:?}", transaction.encode().len());

	let pair = sr25519::Pair::from_string("endorse doctor arch helmet master dragon wild favorite property mercy vault maze", None).unwrap();

	let want = "14e121f6e6cc2891cbbd5f6692e3724672d13e93a3562e3905d4310c2ba1c510 (5CY5hAGk...)";
	let got = format!("{:?}", pair.public());
	assert_eq!(want, got);
	let ans = BlakeTwo256::hash_of(&transaction);

	println!("{:?}", ans);
}
