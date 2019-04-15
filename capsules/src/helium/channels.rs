// Channels 902-928
// 0x0386 ... 0x039F
//
//         906-908 ~ 40k dev
// | Frequency | CF Value | Fract Freq |
// | 906.00000 | 0x038A   | 0x0000     |
// | 906.03986 | 0x038A   | 0x0A34     |
// | 906.07970 | 0x038A   | 0x1467     |
// | 906.12032 | 0x038A   | 0x1ECD     |
// | 906.16016 | 0x038A   | 0x2900     |
// | 906.20001 | 0x038A   | 0x3334     |
// | 906.23987 | 0x038A   | 0x3D67     |
// | 906.27969 | 0x038A   | 0x479A     |
// | 906.32031 | 0x038A   | 0x5200     |
// | 906.36017 | 0x038A   | 0x5C34     |
// | 906.37970 | 0x038A   | 0x6134     |
// | 906.42032 | 0x038A   | 0x6B9A     |
// | 906.46016 | 0x038A   | 0x75CD     |
// | 906.50000 | 0x038A   | 0x8000     |
// | 906.53986 | 0x038A   | 0x8A34     |
// | 906.57970 | 0x038A   | 0x8467     |
// | 906.62032 | 0x038A   | 0x9ECD     |
// | 906.66016 | 0x038A   | 0xA900     |
// | 906.70001 | 0x038A   | 0xB334     |
// | 906.73987 | 0x038A   | 0xBD67     |
// | 906.77969 | 0x038A   | 0xC79A     |
// | 906.82031 | 0x038A   | 0xD200     |
// | 906.86017 | 0x038A   | 0xDC34     |
// | 906.87970 | 0x038A   | 0xE134     |
// | 906.92032 | 0x038A   | 0xEB9A     |
// | 906.96016 | 0x038A   | 0xF5CD     |
// | 907.0000  | 0x038B   | 0x0000     |
// | 907.03986 | 0x038B   | 0x0A34     |
// | 907.07970 | 0x038B   | 0x1467     |
// | 907.12032 | 0x038B   | 0x1ECD     |
// | 907.16016 | 0x038B   | 0x2900     |
// | 907.20001 | 0x038B   | 0x3334     |
// | 907.23987 | 0x038B   | 0x3D67     |
// | 907.27969 | 0x038B   | 0x479A     |
// | 907.32031 | 0x038B   | 0x5200     |
// | 907.36017 | 0x038B   | 0x5C34     |
// | 907.37970 | 0x038B   | 0x6134     |
// | 907.42032 | 0x038B   | 0x6B9A     |
// | 907.46016 | 0x038B   | 0x75CD     |
// | 907.50000 | 0x038B   | 0x8000     |
// | 907.53986 | 0x038B   | 0x8A34     |
// | 907.57970 | 0x038B   | 0x8467     |
// | 907.62032 | 0x038B   | 0x9ECD     |
// | 907.66016 | 0x038B   | 0xA900     |
// | 907.70001 | 0x038B   | 0xB334     |
// | 907.73987 | 0x038B   | 0xBD67     |
// | 907.77969 | 0x038B   | 0xC79A     |
// | 907.82031 | 0x038B   | 0xD200     |
// | 907.86017 | 0x038B   | 0xDC34     |
// | 907.87970 | 0x038B   | 0xE134     |
// | 907.92032 | 0x038B   | 0xEB9A     |
// | 907.96016 | 0x038B   | 0xF5CD     |

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum RadioChannel {
    Channel0,
    Channel1,
    Channel2,
    Channel3,
    Channel4,
    Channel5,
    Channel6,
    Channel7,
    Channel8,
    Channel9,
    Channel10,
    Channel11,
    Channel12,
    Channel13,
    Channel14,
    Channel15,
    Channel16,
    Channel17,
    Channel18,
    Channel19,
    Channel20,
    Channel21,
    Channel22,
    Channel23,
    Channel24,
    Channel25,
    Channel26,
    Channel27,
    Channel28,
    Channel29,
    Channel30,
    Channel31,
    Channel32,
    Channel33,
    Channel34,
    Channel35,
    Channel36,
    Channel37,
    Channel38,
    Channel39,
    Channel40,
    Channel41,
    Channel42,
    Channel43,
    Channel44,
    Channel45,
    Channel46,
    Channel47,
    Channel48,
    Channel49,
    Channel50,
    Channel51,
}

impl RadioChannel {
    pub fn get_channel(&self) -> ChannelParams {
        match *self {
            RadioChannel::Channel0 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x000,
            },
            RadioChannel::Channel1 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x0A34,
            },
            RadioChannel::Channel2 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x1467,
            },
            RadioChannel::Channel3 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x1ECD,
            },
            RadioChannel::Channel4 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x2900,
            },
            RadioChannel::Channel5 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x3334,
            },
            RadioChannel::Channel6 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x3D67,
            },
            RadioChannel::Channel7 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x479A,
            },
            RadioChannel::Channel8 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x5200,
            },
            RadioChannel::Channel9 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x5C34,
            },
            RadioChannel::Channel10 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x6134,
            },
            RadioChannel::Channel11 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x6B9A,
            },
            RadioChannel::Channel12 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x75CD,
            },
            RadioChannel::Channel13 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x8000,
            },
            RadioChannel::Channel14 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x8A34,
            },
            RadioChannel::Channel15 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x8467,
            },
            RadioChannel::Channel16 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0x9ECD,
            },
            RadioChannel::Channel17 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xA900,
            },
            RadioChannel::Channel18 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xB334,
            },
            RadioChannel::Channel19 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xBD67,
            },
            RadioChannel::Channel20 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xC79A,
            },
            RadioChannel::Channel21 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xD200,
            },
            RadioChannel::Channel22 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xDC34,
            },
            RadioChannel::Channel23 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xE134,
            },
            RadioChannel::Channel24 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xEB9A,
            },
            RadioChannel::Channel25 => ChannelParams {
                frequency: 0x038A,
                fract_freq: 0xF5CD,
            },
            RadioChannel::Channel26 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x000,
            },
            RadioChannel::Channel27 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x0A34,
            },
            RadioChannel::Channel28 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x1467,
            },
            RadioChannel::Channel29 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x1ECD,
            },
            RadioChannel::Channel30 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x2900,
            },
            RadioChannel::Channel31 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x3334,
            },
            RadioChannel::Channel32 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x3D67,
            },
            RadioChannel::Channel33 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x479A,
            },
            RadioChannel::Channel34 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x5200,
            },
            RadioChannel::Channel35 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x5C34,
            },
            RadioChannel::Channel36 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x6134,
            },
            RadioChannel::Channel37 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x6B9A,
            },
            RadioChannel::Channel38 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x75CD,
            },
            RadioChannel::Channel39 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x8000,
            },
            RadioChannel::Channel40 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x8A34,
            },
            RadioChannel::Channel41 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x8467,
            },
            RadioChannel::Channel42 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0x9ECD,
            },
            RadioChannel::Channel43 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xA900,
            },
            RadioChannel::Channel44 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xB334,
            },
            RadioChannel::Channel45 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xBD67,
            },
            RadioChannel::Channel46 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xC79A,
            },
            RadioChannel::Channel47 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xD200,
            },
            RadioChannel::Channel48 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xDC34,
            },
            RadioChannel::Channel49 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xE134,
            },
            RadioChannel::Channel50 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xEB9A,
            },
            RadioChannel::Channel51 => ChannelParams {
                frequency: 0x038B,
                fract_freq: 0xF5CD,
            },
        }
    }
}

pub const FREQ_DEVIATIONS: [u16; 26] = [
    0x0000, 0x0A34, 0x1467, 0x1ECD, 0x2900, 0x3334, 0x3D67, 0x479A, 0x5200, 0x5C34, 0x6134, 0x6B9A,
    0x75CD, 0x8000, 0x8A34, 0x8467, 0x9ECD, 0xA900, 0xB334, 0xBD67, 0xC79A, 0xD200, 0xDC34, 0xE134,
    0xEB9A, 0xF5CD,
];

#[derive(Copy, Clone, Default)]
pub struct ChannelParams {
    pub frequency: u16,
    pub fract_freq: u16,
}
