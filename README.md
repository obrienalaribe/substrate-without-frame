## Substrate Frameless Node Template

Welcome to the FRAME-LESS Runtime!

## Account-based Cryptocurrency

### Solution for Assignment
My solution for this assignment is an Account-Based Cryptocurrency Substrate runtime, with a simple Rust CLI user interface.
The commands below will enable you run and test the solution followed by some comments on each requested feature in the assignment

BasicExtrinsic supports the following Calls

```rust
pub enum Call {
	SetValue(u32),
	Transfer(Transaction),
	Mint(Public, Amount),
	Upgrade(Vec<u8>),
}

```
### Instructions

#### Start the Substrate Runtime
Startup the Substrate runtime in a new terminal

```rust
cargo build --release
RUST_LOG=frameless=debug cargo run -- --dev --tmp
```

#### Balance

The below mnemonics are used to create keypairs from the CLI and also used within the Tests.
The Account IDs are pushed into a HashMap with the corresponding Public Key values
```rust
let your_mnemonic = "grass rib slab system month zoo observe render display shallow clay venture";
let johns_mnemonic = "scan true window staff cloth loud accuse retreat tent rack glimpse genuine";

let mut address_map: HashMap<String, Public> = HashMap::new();

```

Go to another terminal and change into the frontend folder where you can run the binary

```bash
cd  frontend && cargo run
```

Type a *single* command **balance** to check the initialized token balance on the 2 Accounts shown below:
```
################################
Your Account: 5CVEC4D99pbkjRUJkSUGZ92Ck2TJ4tBdQgf95f5KAtxxECae
John's Account: 5DRYihD38qKdK1rtVVVo9e73UpaD5vDtxfP3k4HhDgpA2wWB
################################
Please input an action: [mint, transfer, balance]
balance
Please Enter: [Address]
5CVEC4D99pbkjRUJkSUGZ92Ck2TJ4tBdQgf95f5KAtxxECae
Account balance is: 0
Please input an action: [mint, transfer, balance]
mint
```

#### Mint
Mint can be called with an unsigned extrinsic. This provides an easy way to mint balances into accounts on startup from the CLI tool.

```rust
pub enum Call {
	Mint(Public, Amount),
}
```

Type a *single* command **mint** to send some tokens to an Account:

```bash
Please input an action: [mint, transfer, balance]
mint
Please Enter: [toAddress Amount]
5CVEC4D99pbkjRUJkSUGZ92Ck2TJ4tBdQgf95f5KAtxxECae 100
```

#### Transfer
- Transfer must be a signed extrinsic and will fail otherwise. This provides a wallet experience to send tokens across accounts from the CLI tool.

```rust
pub struct BasicExtrinsic{
	call: Call,
	signature: Option<Signature>
}

pub enum Call {
	Transfer(Transaction),
}

pub struct Transaction {
	pub from: Public,
	pub to: Public,
	pub send_amount: Amount,
	pub fee: Amount
}
```

Each transaction body is signed as shown below:
```rust
let message_hash = blake2_256(transaction.encode());
let signature = keypair.sign(&message_hash);
```

#### Block Rewards
- Block rewards are paid to the Alice's on initiialization of every block. A fixed rate of 10 Tokens is issued

#### Fees
- Fees are specified explicitly by the user from the CLI tool
- Fees are sent from the CLI & verified on the Runtime with the below formula

```rust
    if origin_balance >= (transaction.send_amount + transaction.fee) {
       allow transfer
    }
```
