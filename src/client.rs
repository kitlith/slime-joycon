use binrw::{BinRead, BinWrite, binrw, until_eof};

trait PacketType {
    fn packet_type(&self) -> u32;
}

#[binrw]
#[br(import(packet_type: u32))]
enum DevicePacket {
    #[br(pre_assert(packet_type == 0u32))]
    Heartbeat(
        //#[br(temp)]
        //#[bw(calc = 0)]
        u64
    ),
    #[br(pre_assert(packet_type == 1u32))]
    Rotation(Quaternion),
    #[br(pre_assert(packet_type == 2u32))]
    Gyroscope,
    #[br(pre_assert(packet_type == 3u32))]
    Handshake {
        board: u32,
        imu: u32,
        mcu: u32,
        #[br(temp)]
        #[bw(calc = [0; 3])]
        _idk: [u32; 3],
        build_number: u32,
        #[br(temp)]
        #[bw(calc = version.len() as u8)]
        version_size: u8,
        #[br(count = version_size)]
        version: Vec<u8>, // TODO: String
    },
    #[br(pre_assert(packet_type == 4u32))]
    Accelerometer(Vector),
    #[br(pre_assert(packet_type == 5u32))]
    Magnetometer(Vector),
    //#[brw(magic = 6u32)]
    //RawCalibration, // calibration will be stubbed for now
    //#[brw(magic = 7u32)]
    //CalibrationFinished,
    #[br(pre_assert(packet_type == 8u32))]
    Config(DeviceConfig), // TODO
    #[br(pre_assert(packet_type == 9u32))]
    RawMagnetometer(Vector),
    #[br(pre_assert(packet_type == 10u32))]
    PingPong(
        #[br(parse_with = until_eof)]
        Vec<u8>
    ),
    //#[brw(magic = 11u32)]
    //Serial,
    //#[brw(magic = 12u32)]
    //BatteryLevel,
    //#[brw(magic = 13u32)]
    //Tap,
    #[br(pre_assert(packet_type == 14u32))]
    ResetReason,
    #[br(pre_assert(packet_type == 15u32))]
    SensorInfo {
        sensor_id: u8,
        status: u8, // TODO: enum?
    },
    //#[brw(magic = 16u32)]
    //Rotation2,
    #[br(pre_assert(packet_type == 17u32))]
    RotationData(RotationData),
    /*
    #[brw(magic = 18u32)]
    MagnetometerAccuracy {
        sensor_id: u8,
        accuracy: f32
    },
    */
}

impl PacketType for DevicePacket {
    fn packet_type(&self) -> u32 {
        match self {
            DevicePacket::Heartbeat(_) => 0,
            DevicePacket::Rotation(_) => 1,
            DevicePacket::Gyroscope => 2,
            DevicePacket::Handshake { .. } => 3,
            DevicePacket::Accelerometer(_) => 4,
            DevicePacket::Magnetometer(_) => 5,

            DevicePacket::Config(_) => 8,
            DevicePacket::RawMagnetometer(_) => 9,
            DevicePacket::PingPong(_) => 10,

            DevicePacket::ResetReason => 14,
            DevicePacket::SensorInfo { .. } => 15,

            DevicePacket::RotationData(_) => 17
        }
    }
}

#[binrw]
struct RotationData {
    sensor_id: u8,
    data_type: u8, // TODO: enum?
    rotation: Quaternion,
    accuracy: u8 // TODO: enum?
}

#[binrw]
#[br(import(packet_type: u32))]
enum ServerPacket {
    #[br(pre_assert(packet_type == 1u32))]
    Heartbeat,
    #[br(pre_assert(packet_type == 2u32))]
    Vibrate,
    #[br(pre_assert(packet_type == 3u32))]
    Handshake,
    #[br(pre_assert(packet_type == 4u32))]
    Command {
        cmd: u8,
        #[br(parse_with = until_eof)]
        data: Vec<u8>,
    },
    #[br(pre_assert(packet_type == 8u32))]
    SetConfig(DeviceConfig),
    #[br(pre_assert(packet_type == 10u32))]
    PingPong(
        #[br(parse_with = until_eof)]
        Vec<u8>
    ),
    #[br(pre_assert(packet_type == 15u32))]
    SensorInfo,
}

impl PacketType for ServerPacket {
    fn packet_type(&self) -> u32 {
        match self {
            ServerPacket::Heartbeat => 1,
            ServerPacket::Vibrate => 2,
            ServerPacket::Handshake => 3,
            ServerPacket::Command { .. } => 4,
            ServerPacket::SetConfig(_) => 8,
            ServerPacket::PingPong(_) => 10,
            ServerPacket::SensorInfo => 15
        }
    }
}

#[binrw]
#[brw(big)]
struct Packet<T: BinRead<Args=(u32,)> + BinWrite<Args=()> + PacketType> {
    #[br(temp)]
    #[bw(calc = inner.packet_type())]
    packet_type: u32,
    packet_number: u64,
    #[br(args(packet_type))]
    inner: T
}

// NOTE: this structure is little endian, but is entirely byteswapped on the network.
#[binrw]
#[brw(little)]
struct DeviceConfig {
    calibration: CalibrationConfig,
    device_id: u32,
    device_mode: u32
}

/*#[binrw]
struct CalibrationConfig {
    accel_bias: [f32; 3],
    accel_correction: [[f32; 3]; 3],
    mag_bias: [f32; 3],
    mag_correction: [[f32; 3]; 3],
    gyro_bias: [f32; 3]
}*/

#[binrw]
struct CalibrationConfig {
    #[br(map = reverse)]
    #[bw(map = |a| reverse(a.clone()))]
    gyro_bias: [f32; 3],
    #[br(map = reverse_nested)]
    #[bw(map = |a| reverse_nested(a.clone()))]
    mag_correction: [[f32; 3]; 3],
    #[br(map = reverse)] 
    #[bw(map = |a| reverse(a.clone()))]
    mag_bias: [f32; 3],
    #[br(map = reverse_nested)]
    #[bw(map = |a| reverse_nested(a.clone()))]
    accel_correction: [[f32; 3]; 3],
    #[br(map = reverse)]
    #[bw(map = |a| reverse(a.clone()))]
    accel_bias: [f32; 3]
}

fn reverse_mut<T, const N: usize>(arr: &mut [T; N]) {
    for idx in 0..(N/2) {
        arr.swap(idx, N-idx);
    }
}

fn reverse<T, const N: usize>(mut arr: [T; N]) -> [T; N] {
    reverse_mut(&mut arr);
    arr
}

fn reverse_nested<T, const N: usize, const M: usize>(mut arr: [[T; N]; M]) -> [[T; N]; M] {
    reverse_mut(&mut arr);

    for inner in arr.iter_mut() {
        reverse_mut(inner);
    }

    arr
}

enum Command {
    Calibrate, // 1
    SendConfig, // 2
    Blink, // 3
}

#[binrw]
struct Quaternion {
    x: f32,
    y: f32,
    z: f32,
    w: f32
}

#[binrw]
struct Vector {
    x: f32,
    y: f32,
    z: f32,
    #[br(temp)]
    #[bw(calc = 0.0)]
    w: f32
}
