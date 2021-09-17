mod client;

use std::collections::HashMap;

use joycon::{
    hidapi::HidApi,
    JoyCon,
    joycon_sys::{
        NINTENDO_VENDOR_ID, HID_IDS,
        input::BatteryLevel,
        light
    },
    IMU
};

use anyhow::{Context, Result};

use dcmimu::DCMIMU;

use cgmath::{Deg, Euler, One, Quaternion, Vector3};

struct JoyConState {
    battery_level: Option<BatteryLevel>,
    imu_fusion: DCMIMU,
    //orientation: Quaternion<f64>
}

impl Default for JoyConState {
    fn default() -> JoyConState {
        JoyConState {
            battery_level: <_>::default(),
            imu_fusion: DCMIMU::new(),
            //orientation: Quaternion::one()
        }
    }
}

fn main() {
    let api = HidApi::new().unwrap();
    let mut controllers: HashMap<String, (JoyCon, JoyConState)> = HashMap::new();
    loop {
        let new_devices: Vec<_> = api
            .device_list()
            // is joycon
            .filter(|d| d.vendor_id() == NINTENDO_VENDOR_ID && HID_IDS.contains(&d.product_id()))
            // isn't already open
            .filter(|d| !controllers.contains_key(d.serial_number().unwrap()))
            .collect();

        for device_info in new_devices.into_iter() {
            let device = device_info
                .open_device(&api)
                .with_context(|| format!("Error opening the HID device {:?}", device_info))
                .unwrap();

            let mut joycon = match JoyCon::new(device, device_info.clone()) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("Error opening joycon (continuing): {}", e);
                    continue;
                }
            };

            joycon.set_home_light(light::HomeLight::new(
                0x8,
                0x2,
                0x0,
                &[(0xf, 0xf, 0), (0x2, 0xf, 0)],
            )).expect("failed to set home light");

            joycon
                .enable_imu()
                .expect("failed to enable IMU");
            joycon
                .load_calibration()
                .expect("failed to load calibration");
            
            controllers
                .insert(device_info.serial_number().unwrap().to_string(), (joycon, <_>::default()));
        }

        let mut to_remove = Vec::new();

        for (serial, (joycon, state)) in controllers.iter_mut() {
            match handle_joycon(joycon, state) {
                Ok(()) => continue,
                Err(e) => {
                    eprintln!("Removing joycon due to error: {}", e);
                    to_remove.push(serial.to_string());
                }
            }
        }

        for serial in to_remove {
            controllers.remove(&serial).unwrap();
        }
    }
}

fn handle_joycon(joycon: &mut JoyCon, state: &mut JoyConState) -> Result<()> {
    // ... this is blocking, isn't it. multiple joycons is going to involve lots of blocking. hm.
    // Let's hope that the joycon is pushing data to us, instead of us needing to actively poll it.
    // still not ideal though.
    let report = joycon.tick()?;

    let battery_level = report.info.battery_level();
    if Some(battery_level) != state.battery_level {
        state.battery_level = Some(battery_level);

        // NOTE: could just turn lights off to save battery?
        joycon.set_player_light(light::PlayerLights::new(
            (battery_level >= BatteryLevel::Full).into(),
            (battery_level >= BatteryLevel::Medium).into(),
            (battery_level >= BatteryLevel::Low).into(),
            if battery_level >= BatteryLevel::Low {
                light::PlayerLight::On
            } else {
                light::PlayerLight::Blinking
            },
        )).with_context(|| "Error setting joycon lighting.")?;
    }

    for frame in &report.imu.unwrap() {
        // conversion constant from degrees to radians
        const c: f32 = 0.01745329252;
        // conversion constant from G to m/s^2
        const g: f32 = dcmimu::GRAVITY as f32;
        // TODO: consider using a library that uses f64 instead of f32
        state.imu_fusion.update(
            (frame.gyro.x as f32 * c, frame.gyro.y as f32 * c, frame.gyro.z as f32 * c),
            (frame.accel.x as f32 * g, frame.accel.y as f32 * g, frame.accel.z as f32 * g),
            IMU::SAMPLE_DURATION as f32
        );
      
        //state.orientation = state.orientation *
        //    Quaternion::from(Euler::new(
        //        Deg(frame.gyro.y * 0.005),
        //        Deg(frame.gyro.z * 0.005),
        //        Deg(frame.gyro.x * 0.005)
        //    ));
    }
    
    //let euler_orientation = Euler::from(state.orientation);
    //let pitch = Deg::from(euler_orientation.x).0;
    //let yaw = Deg::from(euler_orientation.y).0;
    //let roll = Deg::from(euler_orientation.z).0;

    //println!("pitch: {}, yaw: {}, roll: {}", pitch, yaw, roll);
    println!("Rotation: {:?}", state.imu_fusion.all());
    println!("Bias: {:?}", state.imu_fusion.gyro_biases());

    Ok(())
}
