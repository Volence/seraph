use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Ym2612Write { port: u32, data: u8 },
    Sn76489Write { data: u8 },
    FmKeyOn { channel: u8, operators: u8 },
    FmKeyOff { channel: u8 },
    Panic,
    DacPlayback { samples: Arc<Vec<u8>>, sample_rate: u32 },
    PsgEnvelopePreview { channel: u8, period: u16, envelope: Arc<Vec<u8>>, loop_point: Option<usize> },
    StopPreview,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AudioCommand>();
    }

    #[test]
    fn test_command_clone() {
        let cmd = AudioCommand::Ym2612Write { port: 0, data: 0x42 };
        let cmd2 = cmd.clone();
        match cmd2 {
            AudioCommand::Ym2612Write { port, data } => {
                assert_eq!(port, 0);
                assert_eq!(data, 0x42);
            }
            _ => panic!("wrong variant"),
        }
    }
}
