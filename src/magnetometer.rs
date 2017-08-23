//! Interface to the magnetometer.

use errors::{Error, ErrorKind, Result, ResultExt};
use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CDevice;
use registers;
use std::ops::{Deref, DerefMut};


/// The I2C address of the magnetometer.
const I2C_ADDRESS: u16 = 0x3C >> 1;


/// Interface to an LSM303 digital magnetometer.
pub struct Magnetometer<Dev>
    where Dev: I2CDevice
{
    device: Dev,
    gain: Gain,
}


/// The allowed settings for the gain on the magnetometer.
#[allow(non_camel_case_types)]
pub enum Gain {
    /// +/- 1.3 Gauss
    Gain_1_3,
    /// +/- 1.9 Gauss
    Gain_1_9,
    /// +/- 2.5 Gauss
    Gain_2_5,
    /// +/- 4.0 Gauss
    Gain_4_0,
    /// +/- 4.7 Gauss
    Gain_4_7,
    /// +/- 5,6 Gauss
    Gain_5_6,
    /// +/- 8.1 Gauss
    Gain_8_1,
}


impl Magnetometer<LinuxI2CDevice> {
    /// Initialize the magnetometer for a Linux I2C device.
    pub fn new<Path>(path: Path) -> Result<Magnetometer<LinuxI2CDevice>>
        where Path: AsRef<::std::path::Path>
    {
        let device =
            LinuxI2CDevice::new(&path, I2C_ADDRESS).chain_err(|| ErrorKind::FailedToOpenDevice)?;

        Magnetometer::from_i2c_device(device)
    }
}


impl<Dev> Magnetometer<Dev>
    where Dev: I2CDevice,
          Error: From<Dev::Error>,
          Dev::Error: Send + 'static
{
    /// Initialize the magnetometer, given an open I2C device.
    ///
    /// The opening of the device is platform specific,
    /// but initialization of the sensor is not.
    /// Prefer to use `Accelerometer::new`, unless you are using an
    /// implementation of `I2CDevice` that is not covered by this crate.
    pub fn from_i2c_device(mut device: Dev) -> Result<Magnetometer<Dev>> {
        use registers as r;

        // Set magnetometer to continuous mode
        let mr_reg_m = r::MrRegM::empty();
        write_register!(device, r::MR_REG_M, mr_reg_m)?;

        // enable temperature; set output rate to 15 Hz
        let cra_reg_m = r::TEMP_EN | r::DO2;
        write_register!(device, r::CRA_REG_M, cra_reg_m)?;

        let gain = Gain::Gain_1_3;

        let magnetometer = Magnetometer { device, gain };
        Ok(magnetometer)
    }


    /// Read the magnetometer
    ///
    /// Returns a tuple of (x, y, z).
    /// WIP: the units are unclear.
    pub fn read_magnetic_field(&mut self) -> Result<(i16, i16, i16)> {
        use byteorder::{BigEndian, ReadBytesExt};
        use std::io::Cursor;

        let data = self.device
            .smbus_read_i2c_block_data(registers::OUT_X_H_M, 6)?;
        if data.len() < 6 {
            bail!(ErrorKind::NotEnoughData);
        }

        let mut cursor = Cursor::new(&data);

        // Yes indeed, the registers are ordered as X, Z, Y
        let x = cursor.read_i16::<BigEndian>()?;
        let z = cursor.read_i16::<BigEndian>()?;
        let y = cursor.read_i16::<BigEndian>()?;

        let out = (x, y, z);
        Ok(out)
    }


    /// Set the gain of the magnetometer.
    pub fn set_gain(&mut self, gain: Gain) -> Result<()>
        where Dev::Error: Send + 'static
    {
        use registers::{self as r, CRB_REG_M, CrbRegM};
        let mut flags = read_register!(self.device, CRB_REG_M, CrbRegM)?;

        flags.remove(r::GN2 | r::GN1 | r::GN0);
        let setting = match gain {
            Gain::Gain_1_3 => /* --  |  ---- */ r::GN0,
            Gain::Gain_1_9 => /* -- */ r::GN1,
            Gain::Gain_2_5 => /* -- */ r::GN1 | r::GN0,
            Gain::Gain_4_0 => r::GN2,
            Gain::Gain_4_7 => r::GN2 | /* -- */ r::GN0,
            Gain::Gain_5_6 => r::GN2 | r::GN1,
            Gain::Gain_8_1 => r::GN2 | r::GN1 | r::GN0,
        };
        flags.insert(setting);

        write_register!(self.device, CRB_REG_M, flags)?;
        self.gain = gain;

        Ok(())
    }


    /// Read the thermometer.
    pub fn read_temperature(&mut self) -> Result<i16> {

        // unimplemented!("Not yet ready");

        use byteorder::{BigEndian, ReadBytesExt};
        use std::io::Cursor;

        let data = self.device
            .smbus_read_i2c_block_data(registers::TEMP_OUT_H_M, 2)?;

        let mut cursor = Cursor::new(&data);

        let temp = cursor.read_i16::<BigEndian>()? / 16;

        Ok(temp)
    }
}


/// Access the underlying `I2CDevice`.
///
/// Most of the methods require a mutable reference; `DerefMut` is implemented as well.
impl<Dev> Deref for Magnetometer<Dev>
    where Dev: I2CDevice
{
    type Target = Dev;

    fn deref(&self) -> &Dev {
        &self.device
    }
}


/// Access the underlying I2C device.
///
/// Refer to the LSM303 datasheet if you plan on accessing the device directly.
impl<Dev> DerefMut for Magnetometer<Dev>
    where Dev: I2CDevice
{
    fn deref_mut(&mut self) -> &mut Dev {
        &mut self.device
    }
}
