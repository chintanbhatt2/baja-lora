use core::fmt::Debug;

use heapless::{format, String};
use teensy4_bsp::hal::can::{Data, Frame, Id, MailboxData, StandardId};


#[derive(Clone,)]
pub enum CanMessage {
    Accelerometer(Accelerometer),
    Gyroscope(Gyroscope),
    GPS(GPSModule),
}

impl Debug for CanMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Accelerometer(arg0) => arg0.fmt(f),
            Self::Gyroscope(arg0) => arg0.fmt(f),
            Self::GPS(arg0) => arg0.fmt(f),
        }
    }
}

impl CanMessage {
    pub fn new(data: MailboxData) -> Option<CanMessage> {
        if data.frame.data().is_none() {
            return None;
        }

        let frame_data = data.frame.data().unwrap();
        match data.frame.id() {
            teensy4_bsp::hal::can::Id::Standard(standard_id) => match standard_id.as_raw() {
                0x470 => {
                    if let Some(h) = Gyroscope::new(frame_data) {
                        Some(CanMessage::Gyroscope(h))
                    } else {
                        None
                    }
                }
                0x471 => {
                    if let Some(h) = Accelerometer::new(frame_data) {
                        Some(CanMessage::Accelerometer(h))
                    } else {
                        None
                    }
                }
                0x480 => {
                    if let Some(h) = GPSModule::new(frame_data) {
                        Some(CanMessage::GPS(h))
                    } else {
                        None
                    }
                }
                _ => {
                    log::info!(
                        "ID: {}, data: {:?}",
                        standard_id.as_raw(),
                        data.frame.data()
                    );
                    None
                }
            },
            teensy4_bsp::hal::can::Id::Extended(extended_id) => match extended_id.as_raw() {
                _ => {
                    log::info!("RX: MB{} - {:?}", data.mailbox_number, data.frame);
                    None
                }
            },
        }
    }
}

#[derive(Clone)]
pub struct Accelerometer {
    pub lateral: i16,
    pub longitudinal: i16,
    pub vertical: i16,
}

impl Accelerometer {
    pub fn new(data: &Data) -> Option<Accelerometer> {
        let (chunks, remainder) = data.as_chunks::<2>();
        
        let packet: Accelerometer = Accelerometer {
            lateral: i16::from_be_bytes(chunks[0]),
            longitudinal: i16::from_be_bytes(chunks[1]),
            vertical: i16::from_be_bytes(chunks[2]),
        };

        Some(packet)
    }
}

impl Debug for Accelerometer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(">lat:{}\n>long:{}\n>vert:{}", &self.lateral, &self.longitudinal, &self.vertical))
    }
}

#[derive(Clone)]
pub struct Gyroscope {
    pub yaw: i16,
    pub pitch: i16,
    pub roll: i16,
}

impl Gyroscope {
    pub fn new(data: &Data) -> Option<Gyroscope> {
        let (chunks, remainder) = data.as_chunks::<2>();

        let packet: Gyroscope = Gyroscope {
            yaw: i16::from_be_bytes(chunks[0]),
            pitch: i16::from_be_bytes(chunks[1]),
            roll: i16::from_be_bytes(chunks[2]),
        };

        Some(packet)
    }
}

impl Debug for Gyroscope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(">yaw:{}\n>pitch:{}\n>roll:{}", &self.yaw, &self.pitch, &self.roll))
    }
}


#[derive(Clone, Debug)]
pub struct GPSModule {
    lat: i16,
    long: i16,
    heading: i16,
    speed: i16,
}

impl GPSModule {
    pub fn new(data: &Data) -> Option<GPSModule> {
        let (chunks, remainder) = data.as_chunks::<2>();
        if chunks.len() != 6 {
            return None;
        }

        let packet = GPSModule {
            lat: i16::from_be_bytes(chunks[0]),
            long: i16::from_be_bytes(chunks[1]),
            heading: i16::from_be_bytes(chunks[2]),
            speed: i16::from_be_bytes(chunks[3]),
        };

        Some(packet)
    }
}

impl Into<Data> for GPSModule {
    fn into(self) -> Data {
        let lat = self.lat.to_ne_bytes();
        let long = self.long.to_ne_bytes();
        let heading = self.heading.to_ne_bytes();
        let speed = self.speed.to_ne_bytes();
        let bytes: [u8; 8] = [
            lat[0], lat[1], long[0], long[1], heading[0], heading[1], speed[0], speed[1],
        ];
        let data = Data::new(&bytes);

        if data.is_none() {
            Data::empty()
        } else {
            data.unwrap()
        }
    }
}

#[derive(Clone, Debug)]
pub struct HallEffect {
    pub engine_rpm: i16,
    pub rr_wheel_rpm: i16,
    pub trottle: i16,
}


pub trait TelePort {
    fn to_teleport() -> String<255>;
    fn from_teleport(msg: String<255>) -> Self;
}