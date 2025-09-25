use log::info;
use teensy4_bsp::hal::can::{Data, Frame, Id, MailboxData, StandardId};

#[derive(Clone, Debug)]
pub enum CanMessage{
    RaceGradeIMU(RaceGradeIMU)
}

impl CanMessage {
    pub fn new(data: MailboxData) -> Option<CanMessage>
    {
        match data.frame.id() {
            teensy4_bsp::hal::can::Id::Standard(standard_id) => match standard_id.as_raw() {
                1136 | 1137 => {
                    if let Some(d) = data.frame.data() {
                        if let Some(h) = RaceGradeIMU::new(d) {
                            Some(CanMessage::RaceGradeIMU(h))
                        }
                        else {
                        log::info!("RX: MB{} - {:?}", data.mailbox_number, data.frame);
                            None
                        }
                    } else {
                        log::info!("RX: MB{} - {:?}", data.mailbox_number, data.frame);
                        None
                    }
                },
                _ => {log::info!("ID: {}, data: {:?}", standard_id.as_raw(), data.frame.data()); None},
            },
                teensy4_bsp::hal::can::Id::Extended(extended_id) => match extended_id.as_raw(){
                _ => {log::info!("RX: MB{} - {:?}", data.mailbox_number, data.frame); None}

            },
        }
    }
}


#[derive(Clone, Debug)]
pub struct RaceGradeIMU {
    pub lateral_acc: i16,
    pub longitudinal_acc: i16,
    pub vertical_acc: i16,
    pub yaw_rot: i16,
    pub pitch_rot: i16,
    pub roll_rot: i16
}

impl RaceGradeIMU {
    pub fn new(data: &Data) -> Option<RaceGradeIMU> {
        let (chunks, remainder) = data.as_chunks::<2>();
        if chunks.len() != 6 {
            return None;
        }

        let packet = RaceGradeIMU {
            lateral_acc: i16::from_be_bytes(chunks[0]),
            longitudinal_acc: i16::from_be_bytes(chunks[1]),
            vertical_acc: i16::from_be_bytes(chunks[2]),
            yaw_rot: i16::from_be_bytes(chunks[3]),
            pitch_rot: i16::from_be_bytes(chunks[4]),
            roll_rot: i16::from_be_bytes(chunks[5]),
        };

        Some(packet)
    }

    pub fn zero_message() -> Frame {
        let data: [u8; 8] = [b'i',b'm',b'u',b'z',b'e',b'r',b'o',b'z'];
        Frame::new_data(Id::from(StandardId::new(0x03).unwrap()), data)
    }
}   





