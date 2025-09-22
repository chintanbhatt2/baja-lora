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

#[rtic::app(device = teensy4_bsp, peripherals = true, dispatchers = [KPP])]
mod app {
    use bsp::pins::t40::*;
    use heapless::{format, String};
    use imxrt_log::{self as logging, Poller};
    use sx127x_lora::LoRa;
    use teensy4_bsp::{
        self as bsp,
        board::{self},
        hal::{
            gpio::Output,
            gpt::{self, Gpt},
            timer::Blocking,
        },
        pins::common,
    };

    // If you're using a Teensy 4.1 or MicroMod, you should eventually
    // change 't40' to 't41' or micromod, respectively.
    use board::t40 as my_board;

    use rtic_monotonics::systick::{Systick, *};

    /// There are no resources shared across tasks.
    #[shared]
    struct Shared {
        lora: Option<LoRa<board::Lpspi4, Output<P9>, Output<P6>, Blocking<Gpt<1>, GPT_FREQUENCY>>>,
        can: board::Flexcan2,
    }

    /// These resources are local to individual tasks.
    #[local]
    struct Local {
        /// The LED on pin 1.
        led: Option<Output<common::P13>>,
        /// A poller to control USB logging.
        poller: Option<logging::Poller>,
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
            lpspi4,
            flexcan2,
            mut gpt1,
            ..
        } = my_board(cx.device);

        Systick::start(
            cx.core.SYST,
            board::ARM_FREQUENCY,
            rtic_monotonics::create_systick_token!(),
        );

        let poller_connection = logging::log::usbd(usb, logging::Interrupts::Enabled);

        
        // if let Err(e) = poller_connection {
        //     log::error!("Failed to set up USB logging: {:?}", e);
        //     let led = board::led(&mut gpio2, pins.p13);
        //     blink::spawn().unwrap();

        //     return (
        //         Shared {
        //             lora: None,
        //             flexcan2: None,
        //         },
        //         Local {
        //             led: Some(led),
        //             poller: None,
        //         },
        //     );
        // } else {
            let poller: Option<Poller> = Some(poller_connection.unwrap());
        // }

        // set up the can bus
        let mut can = board::flexcan(flexcan2, pins.p1, pins.p0);
        can.set_baud_rate(1_000_000);
        can.set_max_mailbox(16);
        can.disable_fifo();

        // The LPSPI instance takes pins 10, 11, 12, and 13
        // This means that pin 10 cannot be connected to the LoRa CS pin
        let lpspi4: board::Lpspi4 = board::lpspi(
            lpspi4,
            board::LpspiPins {
                sdo: pins.p11,
                sdi: pins.p12,
                sck: pins.p13,
            },
            1_000_000,
        );

        init_gpt(&mut gpt1);

        // These pins are the ones set up for the lora module
        let reset = gpio2.output(pins.p6);
        let cs = gpio2.output(pins.p9);

        let delay = Blocking::<_, GPT_FREQUENCY>::from_gpt(gpt1);
        let lora = LoRa::new(lpspi4, cs, reset, 915, delay).unwrap();

        // Spawn tasks
        listen_radio::spawn().unwrap();
        run_can_rx::spawn().unwrap();

        (
            Shared {
                lora: Some(lora),
                can,
            },
            Local {
                led: None,
                poller,
            },
        )
    }

    // Keeping this in as sort of a debug function
    // that can also be used as a reference
    #[task(priority=10,local = [led])]
    async fn blink(cx: blink::Context) {
        if let Some(led) = cx.local.led.as_mut() {
            let mut count = 0u32;
            loop {
                led.toggle();
                Systick::delay(500.millis()).await;

                // log::info!("Hello from your Teensy 4! The count is {count}");
                if count.is_multiple_of(7) {
                    // log::warn!("Here's a warning at count {count}");
                }
                if count.is_multiple_of(23) {
                    // log::error!("Here's an error at count {count}");
                }

                count = count.wrapping_add(1);
            }
        }
    }

    // Creates the USB serial poller to connect to a monitor
    #[task(binds = USB_OTG1, local = [poller])]
    fn log_over_usb(cx: log_over_usb::Context) {
        if let Some(poller) = cx.local.poller.as_mut() {
            poller.poll();
        }
    }

    #[task(shared = [lora])]
    async fn transmit_radio(mut cx: transmit_radio::Context, message: String<255>) {
            // The lora object is shared and thus needs to be locked
            // The rest of the syntax in the lock function is a lambda
            // that will execute an arbitrary message send
            cx.shared
                .lora
                .lock(|lora| {
                    if lora.is_none() {
                        log::error!("No LoRa object found!");
                    }

                    let lora = lora.as_mut().unwrap();
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

                });
    }

    #[task(shared = [lora])]
    async fn listen_radio(mut cx: listen_radio::Context) {
        loop {
            cx.shared.lora.lock(|lora| {
                if lora.is_none() {
                    log::error!("No LoRa object found!");
                    return;
                }
                let lora = lora.as_mut().unwrap();
                let poll = lora.poll_irq(Some(30));
                match poll {
                    Ok(size) => {
                        if let Ok(buffer) = lora.read_packet() {
                            let message =
                                core::str::from_utf8(&buffer[..size]).unwrap_or("Invalid UTF-8");
                            log::info!("Recieved Message: {}", message);
                        }
                    }
                    Err(err) => {
                        match err {
                            sx127x_lora::Error::Uninformative => {} // This is for when nothing happens, so just ignore it
                            sx127x_lora::Error::VersionMismatch(v) => {
                                log::error!("Version mismatch: {}", v)
                            }
                            sx127x_lora::Error::CS(c) => log::error!("Chip select error: {}", c),
                            sx127x_lora::Error::Reset(r) => log::error!("Reset error: {}", r),
                            sx127x_lora::Error::SPI(_) => log::error!("SPI error"),
                            sx127x_lora::Error::Transmitting => log::error!("Transmitting error"),
                        }
                    }
                }
            });
            Systick::delay(1.millis()).await;
        }
    }

    #[task(shared = [can])]
    async fn run_can_rx(cx: run_can_rx::Context) {
        let run_can_rx::SharedResources { mut can, .. } = cx.shared;
        loop {
            // read all available mailboxes for any available frames
            if let Some(data) = can.lock(|can| can.read_mailboxes()) {
                log::info!("RX: MB{} - {:?}", data.mailbox_number, data.frame);
                if let Ok(msg) = format!("RX: MB{} - {:?}", data.mailbox_number, data.frame) {
                    let _ = transmit_radio::spawn(msg);
                }
            }

            Systick::delay(250.millis()).await;
        }
    }
}
