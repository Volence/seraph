#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Write a value to a YM2612 register.
    /// port: 0=Part I addr, 1=Part I data, 2=Part II addr, 3=Part II data
    Ym2612Write { port: u32, data: u8 },

    /// Write a byte to the SN76489 register interface.
    Sn76489Write { data: u8 },

    /// Key on: trigger note on an FM channel.
    /// channel: 0-5, operators: bitmask of which ops to enable (0xF0 = all 4)
    FmKeyOn { channel: u8, operators: u8 },

    /// Key off: release note on an FM channel.
    FmKeyOff { channel: u8 },

    /// Stop all sound immediately.
    Panic,
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
