//! Configuration tools

pub mod option;
mod set_get;

pub use crate::enums::ConfigAbilities;
use crate::util::slice_garray;
use crate::SigrokError;
use option::*;
pub(crate) use set_get::*;
use sigrok_sys::{sr_configcap, sr_configkey, sr_dev_config_capabilities_list, sr_dev_options};
use sr_configcap::*;

#[derive(Copy, Clone, Debug)]
struct ConfigPointers {
    driver: *const sigrok_sys::sr_dev_driver,
    sdi: *const sigrok_sys::sr_dev_inst,
    cg: *const sigrok_sys::sr_channel_group,
}

/// The association between a [`Config`] and the type used to configure the option.
///
/// This is implemented by the enums in [`config_items`].
pub trait ConfigAssociation: Copy {
    /// The type that is used to set this option. This is "borrowed" because it is borrowed to set
    /// the option, and owned by the caller—there's no need for this crate to own the configuration
    /// type.
    type BorrowedConfig: SetConfig + ?Sized;
    /// The type that is used to get this option. This is "owned" because it is transferred to the
    /// caller to get the option, as it needs to be created by this crate and then passed to the
    /// caller.
    type OwnedConfig: GetConfig;

    /// The internal key used by Sigrok to identify the config
    fn key(&self) -> u32;
}

pub(crate) mod internal {
    use crate::config::ConfigSetGetPointers;

    pub trait ConfigurablePtr {
        fn ptr(&self) -> ConfigSetGetPointers;
    }
}

/// Controlling the configuration of [`Device`][crate::device::Device]s and their
/// [`ChannelGroup`][crate::device::ChannelGroup]s.
pub trait Configurable: internal::ConfigurablePtr {
    /// Set a configuration. Pass an enum defined in [`config_items`] and its matching type to set
    /// the configuration.
    ///
    /// **You must initialize the [`Device`][crate::device::Device] by
    /// [adding][crate::Session::add_device] it to a [`Session`][crate::Session] before setting a
    /// configuration.**
    ///
    /// ```
    /// use sigrok::{Sigrok, Session};
    /// use sigrok::data::{Datafeed, Logic};
    /// use sigrok::config::{Configurable, config_items};
    ///
    /// # fn main() -> Result<(), sigrok::SigrokError> {
    /// let ctx = Sigrok::new()?;
    /// let sess = Session::new(&ctx)?;
    /// let demo_driver = ctx.drivers().iter().find(|x| x.name() == "demo").unwrap().init()?;
    /// let device = &demo_driver.scan(None)?[0];
    /// sess.add_device(&device)?;
    /// device.config_set(config_items::LimitSamples, &64)?;
    /// # Ok(())
    /// # }
    /// ```
    fn config_set<T: ConfigAssociation>(
        &self,
        config: T,
        value: &T::BorrowedConfig,
    ) -> Result<(), SigrokError> {
        unsafe {
            value.set_config(ConfigSetGetPointers {
                key: config.key(),
                ..self.ptr()
            })
        }
    }
    /// Get a configuration. Pass an enum defined in [`config_items`].
    ///
    /// ```
    /// use sigrok::{Sigrok, Session};
    /// use sigrok::data::{Datafeed, Logic};
    /// use sigrok::config::{Configurable, config_items};
    ///
    /// # fn main() -> Result<(), sigrok::SigrokError> {
    /// let ctx = Sigrok::new()?;
    /// let sess = Session::new(&ctx)?;
    /// let demo_driver = ctx.drivers().iter().find(|x| x.name() == "demo").unwrap().init()?;
    /// let device = &demo_driver.scan(None)?[0];
    /// let sample_rate: u64 = device.config_get(config_items::SampleRate)?;
    /// # Ok(())
    /// # }
    /// ```
    fn config_get<T: ConfigAssociation>(&self, config: T) -> Result<T::OwnedConfig, SigrokError> {
        unsafe {
            T::OwnedConfig::get_config(ConfigSetGetPointers {
                key: config.key(),
                ..self.ptr()
            })
        }
    }
    fn config_options(&self) -> Result<Vec<Config>, SigrokError> {
        unsafe {
            let ptr = self.ptr();
            let arr = sr_dev_options(ptr.driver, ptr.sdi, ptr.cg);
            if arr.is_null() {
                return Err(SigrokError::Err);
            }
            Ok(slice_garray(arr)
                .iter()
                .filter_map(|f: &u32| Config::from_sigrok(*f, ptr.driver, ptr.sdi, ptr.cg))
                .collect())
        }
    }
    fn config_abilities<T: ConfigAssociation>(&self, config: T) -> ConfigAbilities {
        unsafe {
            let ptr = self.ptr();
            let mut abilities = ConfigAbilities::empty();
            let bits = sr_dev_config_capabilities_list(ptr.sdi, ptr.cg, config.key() as i32) as u32;
            abilities.set(ConfigAbilities::GET, bits & SR_CONF_GET as u32 > 0);
            abilities.set(ConfigAbilities::SET, bits & SR_CONF_SET as u32 > 0);
            abilities.set(ConfigAbilities::LIST, bits & SR_CONF_LIST as u32 > 0);
            abilities
        }
    }
}

/// Options used when [scanning for devices][crate::DriverContext::scan]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ScanOption<'a> {
    /// Specification on how to connect to a device.
    ///
    /// In combination with [`SerialComm`][Self::SerialComm], this is a serial port in the
    /// form which makes sense to the OS (e.g., `/dev/ttyS0`). Otherwise this specifies a
    /// USB device, either in the form of
    ///
    /// ```text
    /// <bus>.<address>
    /// ```
    ///
    /// (decimal, e.g. `1.65`) or
    ///
    /// ```text
    /// <vendorid>.<productid>
    /// ```
    ///
    /// (hexadecimal, e.g. `1d6b.0001`).
    Connection(&'a str),
    /// Serial communication specification, in the form:
    ///
    /// ```text
    /// <baudrate>/<databits><parity><stopbits>
    /// ```
    ///
    /// Example: `9600/8n1`
    ///
    /// The string may also be followed by one or more special settings, in the form
    /// `/key=value`. Supported keys and their values are:
    ///
    /// | Key  | Value | Description                            |
    /// |------|-------|----------------------------------------|
    /// | rts  | 0     | set the port's RTS pin to low          |
    /// |      | 1     | set the port's RTS pin to high         |
    /// | dtr  | 0     | set the port's DTR pin to low          |
    /// |      | 1     | set the port's DTR pin to high         |
    /// | flow | 0     | no flow control                        |
    /// |      | 1     | hardware-based (RTS/CTS) flow control  |
    /// |      | 2     | software-based (XON/XOFF) flow control |
    ///
    /// This is always an optional parameter, since a driver typically knows the speed at
    /// which the device wants to communicate.
    SerialComm(&'a str),
    /// Modbus slave address specification.
    ///
    /// This is always an optional parameter, since a driver typically knows the default
    /// slave address of the device.
    ModbusAddr(u64),
}

impl<'a> From<&ScanOption<'a>> for u32 {
    fn from(value: &ScanOption) -> u32 {
        #[deny(unreachable_patterns)]
        match value {
            ScanOption::Connection(_) => sr_configkey::SR_CONF_CONN as u32,
            ScanOption::SerialComm(_) => sr_configkey::SR_CONF_SERIALCOMM as u32,
            ScanOption::ModbusAddr(_) => sr_configkey::SR_CONF_MODBUSADDR as u32,
        }
    }
}

impl<T: internal::ConfigurablePtr> Configurable for T {}
macro_rules! default_value {
    ($a:ty) => {
        $a
    };
    ($a:ty, $b:ty) => {
        $b
    };
}
macro_rules! exclude_nothing {
    ($ty:ty, $($tt:tt)*) => {$($tt)*};
    ($($tt:tt)*) => {};
}
macro_rules! define_values {
    (
	    $(#[$mod_outer:meta])*
        pub mod $mod:ident;
	    $(#[$outer:meta])*
	    pub enum $name:ident from $c_enum:ty as $int:ty {
	        $(
	            $(#[$config_meta:meta])*
	            $config_name:ident$(: $config_type:ty $(| $config_borrowed_type:ty)?)? {
	                $doc:expr,
                    $(
                        $(#[$inner:ident $($args:tt)*])*
                        $c_variant:ident => $variant:ident($value:ty)
                    ),+$(,)?
                }
	        )+
	    }
    ) => {
        $(#[$outer])*
        #[derive(Clone, PartialEq, Debug)]
        pub enum $name {
            $($(
	            $(#[$inner $($args)*])*
                ///
                #[doc = $doc]
	            $variant($value),
            )+)+
        }

        impl $name {
            pub(crate) unsafe fn from_sigrok(
                value: $int,
                driver: *const sigrok_sys::sr_dev_driver,
                sdi: *const sigrok_sys::sr_dev_inst,
                cg: *const sigrok_sys::sr_channel_group,
            ) -> Option<Self> {
                define_consts!(
                    $int,
                    $c_enum,
                    $($($c_variant,)+)+
                );
                #[deny(unreachable_patterns)]
                match value {
                    $($(
                        $c_variant => {
                            Some(if (sr_dev_config_capabilities_list(sdi, cg, value as i32) as u32)
                                & SR_CONF_LIST as u32 > 0 {
                                <$name>::$variant(
                                    <$value>::from_sigrok(value, ConfigPointers { driver, sdi, cg })
                                    .unwrap_or_else(Default::default)
                                )
                            } else {
                                <$name>::$variant(Default::default())
                            })
                        },
                    )+)+
                    _ => None,
                }
            }
		}

        $(#[$mod_outer])*
        pub mod $mod {
            $(
                exclude_nothing! { $($config_type, )?
                    pub use $config_name::*;
                    $(#[$config_meta])*
                    #[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
                    pub enum $config_name {
                        $(
                            $(#[$inner $($args)*])*
                            $variant,
                        )+
                    }
                    impl crate::config::ConfigAssociation for $config_name {
                        type OwnedConfig = $($config_type)?;
                        type BorrowedConfig = $(default_value!($config_type $(,$config_borrowed_type)?))?;

                        fn key(&self) -> u32 { self.into() }
                    }

                    impl std::convert::TryFrom<$int> for $config_name {
                        type Error = ();

                        fn try_from(value: $int) -> Result<Self, Self::Error> {
                            define_consts!(
                                $int,
                                $c_enum,
                                $($c_variant,)+
                            );
                            #[deny(unreachable_patterns)]
                            match value {
                                $($c_variant => Ok($config_name::$variant),)+
                                _ => Err(()),
                            }
                        }
                    }

                    impl From<&$config_name> for $int {
                        fn from(value: &$config_name) -> $int {
                            #[deny(unreachable_patterns)]
                            match value {
                                $($config_name::$variant => <$c_enum>::$c_variant as $int,)+
                            }
                        }
                    }
                }
            )+
		}
    };
}

define_values! {
    /// Typed config items
    ///
    /// These are configuration options organized by type. You may use these with the methods in
    /// the [`Configurable`][crate::config::Configurable] trait, which is implemented by
    /// [`Device`][crate::device::Device] and [`ChannelGroup`][crate::device::ChannelGroup].
    pub mod config_items;
    /// A configuration option and the options it supports, as indicated by
    /// [`Configurable::config_options`].
    ///
    /// Each variant (with the exception of [`TriggerType`][Config::TriggerType]) has a matching
    /// variant in [`config_items`] which may be used to [set][Configurable::config_set] and
    /// [get][Configurable::config_get] its configuration.
    #[non_exhaustive]
    pub enum Config from sigrok_sys::sr_configkey as u32 {
        BoolConfig: bool {
            "This is configurable with a [`bool`].",
            /// The device supports run-length encoding (RLE).
            SR_CONF_RLE => Rle(BoolOption),
            /// The device supports averaging.
            SR_CONF_AVERAGING => Averaging(BoolOption),
            /// Filter.
            SR_CONF_FILTER => Filter(BoolOption),
            /// Max hold mode.
            SR_CONF_HOLD_MAX => HoldMax(BoolOption),
            /// Min hold mode.
            SR_CONF_HOLD_MIN => HoldMin(BoolOption),
            /// The device supports using an external clock.
            SR_CONF_EXTERNAL_CLOCK => ExternalClock(BoolOption),
            /// The device supports swapping channels. Typical this is between buffered and unbuffered channels.
            SR_CONF_SWAP => Swap(BoolOption),
            /// Enabling/disabling channel.
            SR_CONF_ENABLED => Enabled(BoolOption),
            /// Over-voltage protection (OVP) feature
            SR_CONF_OVER_VOLTAGE_PROTECTION_ENABLED => OverVoltageProtectionEnabled(BoolOption),
            /// Over-voltage protection (OVP) active: true if device has activated OVP, i.e. the output voltage exceeds the over-voltage protection threshold.
            SR_CONF_OVER_VOLTAGE_PROTECTION_ACTIVE => OverVoltageProtectionActive(BoolOption),
            /// Over-current protection (OCP) feature
            SR_CONF_OVER_CURRENT_PROTECTION_ENABLED => OverCurrentProtectionEnabled(BoolOption),
            /// Over-current protection (OCP) active: true if device has activated OCP, i.e. the current current exceeds the over-current protection threshold.
            SR_CONF_OVER_CURRENT_PROTECTION_ACTIVE => OverCurrentProtectionActive(BoolOption),
            /// Over-temperature protection (OTP)
            SR_CONF_OVER_TEMPERATURE_PROTECTION => OverTemperatureProtection(BoolOption),
            /// Over-temperature protection (OTP) active.
            SR_CONF_OVER_TEMPERATURE_PROTECTION_ACTIVE => OverTemperatureProtectionActive(BoolOption),
            /// Under-voltage condition.
            SR_CONF_UNDER_VOLTAGE_CONDITION => UnderVoltageCondition(BoolOption),
            /// Under-voltage condition active.
            SR_CONF_UNDER_VOLTAGE_CONDITION_ACTIVE => UnderVoltageConditionActive(BoolOption),
            /// High resolution mode.
            SR_CONF_HIGH_RESOLUTION => HighResolution(BoolOption),
            /// Peak detection.
            SR_CONF_PEAK_DETECTION => PeakDetection(BoolOption),
            /// Power off the device.
            SR_CONF_POWER_OFF => PowerOff(BoolOption),
            /// The device supports continuous sampling. Neither a time limit nor a sample number limit has to be supplied, it will just acquire samples continuously, until explicitly stopped by a certain command.
            SR_CONF_CONTINUOUS => Continuous(BoolOption),
            /// The device has internal storage, into which data is logged. This starts or stops the internal logging.
            SR_CONF_DATALOG => Datalog(BoolOption),
        }
        StringConfig: String | str {
            "This is configurable with a `&`[`str`] or [`String`].",
            /// The device supports setting a pattern (pattern generator mode).
            SR_CONF_PATTERN_MODE => PatternMode(StringOption),
            /// The device supports setting trigger slope.
            SR_CONF_TRIGGER_SLOPE => TriggerSlope(StringOption),
            /// Trigger source.
            SR_CONF_TRIGGER_SOURCE => TriggerSource(StringOption),
            /// Coupling.
            SR_CONF_COUPLING => Coupling(StringOption),
            /// Sound pressure level frequency weighting.
            SR_CONF_SPL_WEIGHT_FREQ => SplWeightFreq(StringOption),
            /// Sound pressure level time weighting.
            SR_CONF_SPL_WEIGHT_TIME => SplWeightTime(StringOption),
            /// Channel configuration.
            SR_CONF_CHANNEL_CONFIG => ChannelConfig(StringOption),
            /// Choice of clock edge for external clock ("r" or "f").
            SR_CONF_CLOCK_EDGE => ClockEdge(StringOption),
            /// Channel regulation get: "CV", "CC" or "UR", denoting constant voltage, constant current or unregulated. "CC-" denotes a power supply in current sink mode (e.g. HP 66xxB). "" is used when there is no regulation, e.g. the output is disabled.
            SR_CONF_REGULATION => Regulation(StringOption),
            /// Equivalent circuit model.
            SR_CONF_EQUIV_CIRCUIT_MODEL => EquivCircuitModel(StringOption),
            /// Which external clock source to use if the device supports multiple external clock channels.
            SR_CONF_EXTERNAL_CLOCK_SOURCE => ExternalClockSource(StringOption),
            /// The device supports setting a pattern for the logic trigger.
            SR_CONF_TRIGGER_PATTERN => TriggerPattern(StringOption),
            /// Logic threshold: predefined levels (TTL, ECL, CMOS, etc).
            SR_CONF_LOGIC_THRESHOLD => LogicThreshold(StringOption),
            /// The measurement range of a DMM or the output range of a power supply.
            SR_CONF_RANGE => Range(StringOption),
            /// The number of digits (e.g. for a DMM).
            SR_CONF_DIGITS => Digits(StringOption),
            /// Session filename.
            SR_CONF_SESSIONFILE => SessionFile(StringOption),
            /// The device supports specifying a capturefile to inject.
            SR_CONF_CAPTUREFILE => CaptureFile(StringOption),
            /// Data source for acquisition. If not present, acquisition from the device is always "live", i.e. acquisition starts when the frontend asks and the results are sent out as soon as possible. If present, it indicates that either the device has no live acquisition capability (for example a pure data logger), or there is a choice. In any case if a device has live acquisition capabilities, it is always the default.
            SR_CONF_DATA_SOURCE => DataSource(StringOption),
            /// Device mode for multi-function devices.
            SR_CONF_DEVICE_MODE => DeviceMode(StringOption),
            /// Self test mode.
            SR_CONF_TEST_MODE => TestMode(StringOption),
        }
        U64RangeConfig: std::ops::RangeInclusive<u64> {
            "This is configurable with a [`RangeInclusive`][std::ops::RangeInclusive]`<`[`u64`]`>`.",
            /// Sound pressure level measurement range.
            SR_CONF_SPL_MEASUREMENT_RANGE => SplMeasurementRange(U64RangeOption),
        }
        F64RangeConfig: std::ops::RangeInclusive<f64> {
            "This is configurable with a [`RangeInclusive`][std::ops::RangeInclusive]`<`[`f64`]`>`.",
            /// Logic low-high threshold range.
            SR_CONF_VOLTAGE_THRESHOLD => VoltageThreshold(F64RangeOption),
        }
        U64Config: u64 {
            "This is configurable with an [`u64`].",
            /// The device supports setting its samplerate, in Hz.
            SR_CONF_SAMPLERATE => SampleRate(SampleRateOption),
            /// The device supports setting a pre/post-trigger capture ratio.
            SR_CONF_CAPTURE_RATIO => CaptureRatio(U64Option),
            /// The device supports setting number of samples to be averaged over.
            SR_CONF_AVG_SAMPLES => AvgSamples(U64Option),
            /// Buffer size.
            SR_CONF_BUFFERSIZE => BufferSize(U64Option),
            /// The device supports setting its sample interval, in ms.
            SR_CONF_SAMPLE_INTERVAL => SampleInterval(U64Option),
            /// Center frequency. The input signal is downmixed by this frequency before the ADC anti-aliasing filter.
            SR_CONF_CENTER_FREQUENCY => CenterFrequency(U64Option),
            /// The device supports specifying the capturefile unit size.
            SR_CONF_CAPTURE_UNITSIZE => CaptureUnitSize(U64Option),
            /// The device supports setting a probe factor.
            SR_CONF_PROBE_FACTOR => ProbeFactor(U64Option),
            /// The device supports setting a sample time limit (how long the sample acquisition should run, in ms).
            SR_CONF_LIMIT_MSEC => LimitMsec(U64Option),
            /// The device supports setting a sample number limit (how many samples should be acquired).
            SR_CONF_LIMIT_SAMPLES => LimitSamples(U64Option),
            /// The device supports setting a frame limit (how many frames should be acquired).
            SR_CONF_LIMIT_FRAMES => LimitFrames(U64Option),
        }
        F64Config: f64 {
            "This is configurable with a [`f64`].",
            /// Horizontal trigger position.
            SR_CONF_HORIZ_TRIGGERPOS => HorizTriggerpos(F64Option),
            /// Current voltage.
            SR_CONF_VOLTAGE => Voltage(F64Option),
            /// Maximum target voltage.
            SR_CONF_VOLTAGE_TARGET => VoltageTarget(F64Option),
            /// Current current.
            SR_CONF_CURRENT => Current(F64Option),
            /// Current limit.
            SR_CONF_CURRENT_LIMIT => CurrentLimit(F64Option),
            /// Over-voltage protection (OVP) threshold
            SR_CONF_OVER_VOLTAGE_PROTECTION_THRESHOLD => OverVoltageProtectionThreshold(F64Option),
            /// Over-current protection (OCP) threshold
            SR_CONF_OVER_CURRENT_PROTECTION_THRESHOLD => OverCurrentProtectionThreshold(F64Option),
            /// Amplitude of a source without strictly-defined MQ.
            SR_CONF_AMPLITUDE => Amplitude(F64Option),
            /// Output frequency in Hz.
            SR_CONF_OUTPUT_FREQUENCY => OutputFrequency(F64Option),
            /// Output frequency target in Hz.
            SR_CONF_OUTPUT_FREQUENCY_TARGET => OutputFrequencyTarget(F64Option),
            /// Trigger level.
            SR_CONF_TRIGGER_LEVEL => TriggerLevel(F64Option),
            /// Under-voltage condition threshold.
            SR_CONF_UNDER_VOLTAGE_CONDITION_THRESHOLD => UnderVoltageConditionThreshold(F64Option),
            /// Offset of a source without strictly-defined MQ.
            SR_CONF_OFFSET => Offset(F64Option),
            /// Logic threshold: custom numerical value.
            SR_CONF_LOGIC_THRESHOLD_CUSTOM => LogicThresholdCustom(F64Option),
            /// Number of powerline cycles for ADC integration time.
            SR_CONF_ADC_POWERLINE_CYCLES => AdcPowerlineCycles(F64Option),
        }
        I32Config: i32 {
            "This is configurable with an [`i32`].",
            /// Number of horizontal divisions, as related to SR_CONF_TIMEBASE.
            SR_CONF_NUM_HDIV => NumHdiv(I32Option),
            /// Number of vertical divisions, as related to SR_CONF_VDIV.
            SR_CONF_NUM_VDIV => NumVdiv(I32Option),
            /// The device supports setting the number of logic channels.
            SR_CONF_NUM_LOGIC_CHANNELS => NumLogicChannels(I32Option),
            /// The device supports setting the number of analog channels.
            SR_CONF_NUM_ANALOG_CHANNELS => NumAnalogChannels(I32Option),
        }
        RationalConfig: num_rational::Ratio<u64> {
            "This is configurable with a [`Ratio`][num_rational::Ratio]`<`[`u64`]`>`.",
            /// Time base.
            SR_CONF_TIMEBASE => Timebase(RationalOption),
            /// Volts/div.
            SR_CONF_VDIV => Vdiv(RationalOption),
        }
        TriggerConfig {
            "This is not configurable in a normal manner. You must control that with
            [`Triggers`][crate::Triggers].",
            /// Trigger matches.
            SR_CONF_TRIGGER_MATCH => TriggerType(TriggerOption),
        }
        MqConfig: crate::data::Mq {
            "This is configurable with a [`Mq`][crate::data::Mq].",
            /// Measured quantity.
            SR_CONF_MEASURED_QUANTITY => MeasuredQuantity(MqOption),
        }
    }
}
