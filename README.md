# Setup Instructions
## 1. Install Rust
I will have a .hex file that's the latest working version up on the github, but in case you want to build it from scratch go here:

https://www.rust-lang.org/learn/get-started
## 2. Install Rust Tool Chains
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
## 3. Compile
Navigate to where you cloned this repo and type

```
cargo build --release
```
This will take a WHILE if this is your first time building. Don't worry, after this initial run, compile times should only take a couple seconds.
## 4. Generate the HEX file

This is what you're going to use to upload to the Teensy4
Paste the following command into your terminal
```
cargo objcopy --release -- -O ihex baja-lora.hex
```
## 5. Upload the HEX file to the Teensy
If you have a preferred way of uploading files to the board, go for it. If you don't I would recommend downloading this tool: https://www.pjrc.com/teensy/loader.html
# Why Rust?
There's plenty of reasons for choosing Rust over C++ or python.
## Safety

Out of the box, Rust guarantees memory safety. What does that mean? In C++, you can generate a pointer and delete the memory address which can lead to the microcontroller crashing. Memory safety can also mean safety from race conditions. Take the following code as an example.

```cpp
#include <SPI.h>
const int chipSelectPin = 10;

void setup() {
  pinMode (chipSelectPin, OUTPUT);
  digitalWrite (chipSelectPin, HIGH);
  SPI.begin(); 
}

void loop() {
  for (int channel = 0; channel < 6; channel++) { 
    for (int level = 0; level < 255; level++) {
      digitalPotWrite(channel, level);
      delay(10);
    }
    delay(100);
    for (int level = 0; level < 255; level++) {
      digitalPotWrite(channel, 255 - level);
      delay(10);
    }
    blink_led();
  }

}

void blink_led()
{
	digitalWrite(LED_BUILTIN, HIGH);
	digitalWrite(LED_BUILTIN, LOW)
}

void digitalPotWrite(int address, int value) {
  SPI.beginTransaction(SPISettings(1000000, MSBFIRST, SPI_MODE1));
  digitalWrite(slaveSelectPin,LOW);
  SPI.transfer(address);
  SPI.transfer(value);
  digitalWrite(slaveSelectPin,HIGH);
  SPI.endTransaction();
}

```

This will set up the SPI communication on the Teensy as well as blink the LED once everything is done. Sounds like there's no problems there.

The only issue is that pin 13 is also used for SPI communication, but C++ is happy to let you compile and run the program all day while you scratch your head to figure out why you're not getting any communication.

With Rust however,

```rust
fn main()
{
        let lpspi4: board::Lpspi4 = board::lpspi(
            lpspi4,
            board::LpspiPins {
                sdo: pins.p11,
                sdi: pins.p12,
                sck: pins.p13,
                pcs0: pins.p10,
            },
            1_000_000,
        );
        let led = gpio2.output(pins.p13); // ERROR: use of moved value: `pins.p13`
}

```
Even before compiling, my code editor will warn me that I've already assigned pin 13 to something else and will not let my program compile until another pin is used. 

While these are simple examples, once a project gets more complicated it's nice to have assurance that once your code compiles, it will won't crash in silly ways.

Type safety will also help stop you from making silly mistakes. While C++ has types, they're more of a suggestion and you're able to do whatever you want to the types in order to fit your program. Here, the type system will guide you on what you may be doing wrong, what pins aren't allowed to be assigned to certain peripherals, or if something is wrong with your internal routing.
## Runtime
The RTIC run time offers insane flexibility for how we're able to increase the complexity of our telemetry. Because of the runtime you would be able to create endless functionality with the two-way communication, with plenty of opportunity to expand the system to fit whatever other equipment it needs to interface with. The automatic scheduling and async management out of the box will make it a breeze to swap functions in and out not only at run-time, but at compile time too.
If we need to make specialty Teensys we would only need one program that can handle creating whatever kind of controller we need. One program means we can have a million copies of every device in case something goes wrong.
## I don't trust you
Well, it's not that I don't trust you, but rather that Rust makes it easier for the maintainer to not have to check your code. If your code compiles, then it's most likely not going to have any programming specific issues at runtime. It won't stop you from writing wrong code, but it will stop you from writing code that crashes. This makes it easier to onboard new people onto the code base since the compiler will be there every step of the way to guide them to success. Plus, learning new languages never hurt nobody so give Rust a try and you might be surprised at how helpful it can be.
