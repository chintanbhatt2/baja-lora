#Setup Instructions

# 1. Install Rust

I will have a .hex file that's the latest working version up on the github, but in case you want to build it from scratch go here:
https://www.rust-lang.org/learn/get-started

# 2. Install Rust Tool Chains

You'll need the target for the Teensy4 architechture. After installing rust, restart your computer and paste this in your terminal
```
rustup target add thumbv7em-none-eabihf
```
then paste these instructions
```
cargo install cargo-binutils
```
```
rustup component add llvm-tools
```

# 3. Compile

Navigate to where you cloned this repo and type

```
cargo build --release
```
This will take a WHILE if this is your first time building. Don't worry, after this initial run, compile times should only take a couple seconds.

# 4. Generate the HEX file

This is what you're going to use to upload to the Teensy4
Paste the following command into your terminal
```
cargo objcopy --release -- -O ihex baja-lora.hex
```

# 5. Upload the HEX file to the Teensy

If you have a preferred way of uploading files to the board, go for it. If you don't I would recommend downloading this tool: https://www.pjrc.com/teensy/loader.html
