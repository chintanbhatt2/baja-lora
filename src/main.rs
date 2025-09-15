//! The starter code slowly blinks the LED and sets up
//! USB logging. It periodically logs messages over USB.
//!
//! Despite targeting the Teensy 4.0, this starter code
//! should also work on the Teensy 4.1 and Teensy MicroMod.
//! You should eventually target your board! See inline notes.
//!
//! This template uses [RTIC v2](https://rtic.rs/2/book/en/)
//! for structuring the application.

#![no_std]
#![no_main]

use teensy4_panic as _;

#[rtic::app(device = teensy4_bsp, peripherals = true, dispatchers = [KPP, PIT])]
mod app {

    use bsp::hal::gpt;
    use imxrt_log as logging;
    use sx127x_lora::LoRa;
    use teensy4_bsp::{
        self as bsp, board,
        hal::{gpt::Gpt, timer::Blocking},
    };

    // If you're using a Teensy 4.1 or MicroMod, you should eventually
    // change 't40' to 't41' or micromod, respectively.
    use board::t40 as my_board;

    use rtic_monotonics::systick::{Systick, *};

    /// There resources are shared across tasks.
    #[shared]
    struct Shared {
        // The lora module
        lora: LoRa<
            board::Lpspi4,
            bsp::hal::gpio::Output<bsp::pins::t40::P0>,
            bsp::hal::gpio::Output<bsp::pins::t40::P1>,
            Blocking<Gpt<1>, GPT_FREQUENCY>,
        >,
    }

    /// These resources are local to individual tasks.
    #[local]
    struct Local {
        /// The LED on pin 13.
        led: board::Led,
        /// A poller to control USB logging.
        poller: logging::Poller,
    }

    // Given this GPT clock source...
    const GPT_CLOCK_SOURCE: gpt::ClockSource = gpt::ClockSource::PeripheralClock;
    // ...and this GPT-specific divider...
    const GPT_DIVIDER: u32 = 8;
    /// ...the GPT frequency is
    const GPT_FREQUENCY: u32 = board::PERCLK_FREQUENCY / GPT_DIVIDER;

    fn init_gpt<const N: u8>(gpt: &mut gpt::Gpt<N>) {
        gpt.set_clock_source(GPT_CLOCK_SOURCE);
        gpt.set_divider(GPT_DIVIDER);
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let board::Resources {
            mut gpio2,
            pins,
            usb,
            mut gpt1,
            mut gpio1,
            ..
        } = my_board(cx.device);

        let led = board::led(&mut gpio2, pins.p13);

        let board::T40Resources { lpspi4, pins, .. } = board::t40(board::instances());

        // Set up the LPSPI4 peripheral with 1 MHz clock
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

        let poller = logging::log::usbd(usb, logging::Interrupts::Enabled).unwrap();

        Systick::start(
            cx.core.SYST,
            board::ARM_FREQUENCY,
            rtic_monotonics::create_systick_token!(),
        );

        // let cs = board::instances().GPIO1.into();

        //Set the lora pins
        let cs = gpio1.output(pins.p0);
        let reset = gpio1.output(pins.p1);
        // create a gpt timer for delay
        init_gpt(&mut gpt1);

        // Create a blocking delay using GPT1
        let delay = Blocking::<_, GPT_FREQUENCY>::from_gpt(gpt1);

        let lora = LoRa::new(lpspi4, cs, reset, 915, delay).unwrap();

        // Async systems
        blink::spawn().unwrap();
        transmit_radio::spawn().unwrap();
        listen_radio::spawn().unwrap();

        (Shared { lora }, Local { led, poller })
    }

    #[task(priority = 2, shared = [lora])]
    async fn transmit_radio(mut cx: transmit_radio::Context) {
        loop {
            cx.shared
                .lora
                .lock(|lora| {
                    // transmit a message every 10 seconds
                    let message = "TEST IN PROGRESS!";
                    let mut buffer = [0; 255];
                    for (i, c) in message.chars().enumerate() {
                        buffer[i] = c as u8;
                    }

                    match lora.transmit_payload(buffer, message.chars().count()) {
                        Ok(_) => {
                            log::info!("Sent message: {:?}", message);
                        }
                        Err(e) => {
                            log::error!("Failed to send message: {:?}", e);
                        }
                    }

                    Systick::delay(10.secs())
                })
                .await;
        }
    }

    #[task(priority = 1, shared = [lora])]
    async fn listen_radio(mut cx: listen_radio::Context) {
        loop {
            cx.shared.lora.lock(|lora| match lora.poll_irq(None) {
                Ok(size) => {
                    if let Ok(buffer) = lora.read_packet() {
                        if size > 0 {
                            if let Ok(text) = core::str::from_utf8(&buffer[..size]) {
                                log::info!("Received message: {:?}", text);
                            } else {
                                log::warn!("Received non-UTF8 message: {:?}", &buffer[..size]);
                            }
                        }
                    }
                }
                Err(_) => unreachable!(),
            });
        }
    }

    #[task(binds = USB_OTG1, local = [poller])]
    fn log_over_usb(cx: log_over_usb::Context) {
        cx.local.poller.poll();
    }

    #[task(local = [led])]
    async fn blink(cx: blink::Context) {
        let mut count = 0u32;
        loop {
            cx.local.led.toggle();

            Systick::delay(500.millis()).await;

            log::info!("Hello from your Teensy 4! The count is {count}");
            if count.is_multiple_of(7) {
                log::warn!("Here's a warning at count {count}");
            }
            if count.is_multiple_of(23) {
                log::error!("Here's an error at count {count}");
            }

            count = count.wrapping_add(1);
        }
    }
}
