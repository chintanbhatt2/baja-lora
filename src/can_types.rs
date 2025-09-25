use teensy4_bsp::hal::can::{Data, Frame, Id, MailboxData, StandardId};

#[derive(Clone, Debug)]
pub enum CanMessage {
    Accelerometer(Accelerometer),
    Gyroscope(Gyroscope),
    GPS(GPSModule),
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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
