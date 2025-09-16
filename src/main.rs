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
    use sx127x_lora::LoRa;
    use teensy4_bsp::{self as bsp, board, hal::{gpio::Output, gpt::{self, Gpt}, iomuxc::Pad, lpspi::Lpspi, timer::Blocking}, pins::common};

    use imxrt_log as logging;

    // If you're using a Teensy 4.1 or MicroMod, you should eventually
    // change 't40' to 't41' or micromod, respectively.
    use board::t40 as my_board;

    use rtic_monotonics::systick::{fugit::Duration, Systick, *};

    /// There are no resources shared across tasks.
    #[shared]
    struct Shared {
        lora: LoRa<
                board::Lpspi4,
                bsp::hal::gpio::Output<bsp::pins::t40::P9>,
                bsp::hal::gpio::Output<bsp::pins::t40::P6>,
                Blocking<Gpt<1>, GPT_FREQUENCY>,
            >,
    }

    /// These resources are local to individual tasks.
    #[local]
    struct Local {
        /// The LED on pin 13.
        led: Output<common::P1>,
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
            mut gpio1,
            pins,
            usb,
            lpspi4,
            mut gpt1,
            ..
        } = my_board(cx.device);

        let led = gpio1.output(pins.p1);
        let poller = logging::log::usbd(usb, logging::Interrupts::Enabled).unwrap();

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

        init_gpt(&mut gpt1);
        let reset = gpio2.output(pins.p6);
        let cs = gpio2.output(pins.p9);

        let delay = Blocking::<_, GPT_FREQUENCY>::from_gpt(gpt1);
        let lora = LoRa::new(lpspi4, cs, reset, 915, delay).unwrap();


        Systick::start(
            cx.core.SYST,
            board::ARM_FREQUENCY,
            rtic_monotonics::create_systick_token!(),
        );
        
        blink::spawn().unwrap();
        transmit_radio::spawn().unwrap();
        listen_radio::spawn().unwrap();

        (Shared {lora }, Local { led, poller })
    }

    #[task(priority=10,local = [led])]
    async fn blink(cx: blink::Context) {
        let mut count = 0u32;
        loop {
            cx.local.led.toggle();
            Systick::delay(500.millis()).await;

            // log::info!("Hello from your Teensy 4! The count is {count}");
            if count % 7 == 0 {
                // log::warn!("Here's a warning at count {count}");
            }
            if count % 23 == 0 {
                // log::error!("Here's an error at count {count}");
            }

            count = count.wrapping_add(1);
        }
    }

    #[task(binds = USB_OTG1, local = [poller])]
    fn log_over_usb(cx: log_over_usb::Context) {
        cx.local.poller.poll();
    }

    #[task(priority=10,shared = [lora])]
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

    #[task(shared = [lora])]
    async fn listen_radio(mut cx: listen_radio::Context) {
        loop {
            cx.shared.lora.lock(|lora| {
                let poll = lora.poll_irq(Some(30));
                match poll {
                    Ok(size) => {
                        log::info!("with payload: ");
                        if let Ok(buffer) = lora.read_packet(){
                                let message = core::str::from_utf8(&buffer[..size]).unwrap_or("Invalid UTF-8");
                                log::info!("{}", message);
                        }
                        
                    },
                    Err(err) => {
                        match err {
                            sx127x_lora::Error::Uninformative => {},
                            sx127x_lora::Error::VersionMismatch(v) => log::error!("Version mismatch: {}", v),
                            sx127x_lora::Error::CS(c) => log::error!("Chip select error: {}", c),
                            sx127x_lora::Error::Reset(r) => log::error!("Reset error: {}", r),
                            sx127x_lora::Error::SPI(_) => log::error!("SPI error"),
                            sx127x_lora::Error::Transmitting => log::error!("Transmitting error"),
                        }

                    }
                }
            });
        }
    }

}

