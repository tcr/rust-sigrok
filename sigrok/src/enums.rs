use sigrok_sys::sr_mqflag;
bitflags::bitflags! {
    pub struct MqFlags: u32 {
        /// Voltage measurement is alternating current (AC).
        const AC = sr_mqflag::SR_MQFLAG_AC.0;
        /// Voltage measurement is direct current (DC).
        const DC = sr_mqflag::SR_MQFLAG_DC.0;
        /// This is a true RMS measurement.
        const RMS = sr_mqflag::SR_MQFLAG_RMS.0;
        /// Value is voltage drop across a diode, or NAN.
        const DIODE = sr_mqflag::SR_MQFLAG_DIODE.0;
        /// Device is in "hold" mode (repeating the last measurement).
        const HOLD = sr_mqflag::SR_MQFLAG_HOLD.0;
        /// Device is in "max" mode, only updating upon a new max value.
        const MAX = sr_mqflag::SR_MQFLAG_MAX.0;
        /// Device is in "min" mode, only updating upon a new min value.
        const MIN = sr_mqflag::SR_MQFLAG_MIN.0;
        /// Device is in autoranging mode.
        const AUTORANGE = sr_mqflag::SR_MQFLAG_AUTORANGE.0;
        /// Device is in relative mode.
        const RELATIVE = sr_mqflag::SR_MQFLAG_RELATIVE.0;
        /// Sound pressure level is A-weighted in the frequency domain, according to IEC 61672:2003.
        const SPL_FREQ_WEIGHT_A = sr_mqflag::SR_MQFLAG_SPL_FREQ_WEIGHT_A.0;
        /// Sound pressure level is C-weighted in the frequency domain, according to IEC 61672:2003.
        const SPL_FREQ_WEIGHT_C = sr_mqflag::SR_MQFLAG_SPL_FREQ_WEIGHT_C.0;
        /// Sound pressure level is Z-weighted (i.e. not at all) in the frequency domain, according to IEC 61672:2003.
        const SPL_FREQ_WEIGHT_Z = sr_mqflag::SR_MQFLAG_SPL_FREQ_WEIGHT_Z.0;
        /// Sound pressure level is not weighted in the frequency domain, albeit without standards-defined low and high frequency limits.
        const SPL_FREQ_WEIGHT_FLAT = sr_mqflag::SR_MQFLAG_SPL_FREQ_WEIGHT_FLAT.0;
        /// Sound pressure level measurement is S-weighted (1s) in the time domain.
        const SPL_TIME_WEIGHT_S = sr_mqflag::SR_MQFLAG_SPL_TIME_WEIGHT_S.0;
        /// Sound pressure level measurement is F-weighted (125ms) in the time domain.
        const SPL_TIME_WEIGHT_F = sr_mqflag::SR_MQFLAG_SPL_TIME_WEIGHT_F.0;
        /// Sound pressure level is time-averaged (LAT), also known as Equivalent Continuous A-weighted Sound Level (LEQ).
        const SPL_LAT = sr_mqflag::SR_MQFLAG_SPL_LAT.0;
        /// Sound pressure level represented as a percentage of measurements that were over a preset alarm level.
        const SPL_PCT_OVER_ALARM = sr_mqflag::SR_MQFLAG_SPL_PCT_OVER_ALARM.0;
        /// Time is duration (as opposed to epoch, ...).
        const DURATION = sr_mqflag::SR_MQFLAG_DURATION.0;
        /// Device is in "avg" mode, averaging upon each new value.
        const AVG = sr_mqflag::SR_MQFLAG_AVG.0;
        /// Reference value shown.
        const REFERENCE = sr_mqflag::SR_MQFLAG_REFERENCE.0;
        /// Unstable value (hasn't settled yet).
        const UNSTABLE = sr_mqflag::SR_MQFLAG_UNSTABLE.0;
        /// Measurement is four wire (e.g. Kelvin connection).
        const FOUR_WIRE = sr_mqflag::SR_MQFLAG_FOUR_WIRE.0;
    }
}
bitflags::bitflags! {
    /// The abilities of a config item, in terms of getting, setting, and listing all possible
    /// options.
    ///
    /// This is returned by
    /// [`Configurable::config_options`][crate::config::Configurable::config_options].
    pub struct ConfigAbilities: u8 {
        /// Get the current value.
        const GET = 1;
        /// Set the config to a new value.
        const SET = 2;
        /// List the possible options with `config_options`.
        const LIST = 4;
    }
}
/// Measured Quantity
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Mq {
    pub mq_type: MqType,
    pub flags: MqFlags,
}
macro_rules! define_enum {
    (
	    $(#[$outer:meta])*
	    pub enum $name:ident from $c_enum:ty as $int:ty {
	        $(
	            $(#[$inner:ident $($args:tt)*])*
	            $c_variant:ident => $variant:ident
	        ),+$(,)?
	    }
    ) => {
        $(#[$outer])*
        #[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
        pub enum $name {
            $(
	            $(#[$inner $($args)*])*
	            $variant,
            )+
        }

        impl std::convert::TryFrom<$int> for $name {
            type Error = ();

            fn try_from(value: $int) -> Result<Self, Self::Error> {
                define_consts!(
                    $int,
                    $c_enum,
                    $($c_variant,)+
                );
                #[deny(unreachable_patterns)]
                match value {
                    $($c_variant => Ok($name::$variant),)+
                    _ => Err(()),
                }
            }
		}

		impl From<$name> for $int {
		    fn from(value: $name) -> $int {
                #[deny(unreachable_patterns)]
		        match value {
		            $($name::$variant => <$c_enum>::$c_variant as $int,)+
		        }
		    }
		}
    };
}
define_enum! {
    /// Unit of measured quantity. Used in [`Analog::unit`][crate::data::Analog::unit].
    #[non_exhaustive]
    pub enum Unit from sigrok_sys::sr_unit as u32 {
        /// Volt
        SR_UNIT_VOLT => Volt,
        /// Ampere (current).
        SR_UNIT_AMPERE => Ampere,
        /// Ohm (resistance).
        SR_UNIT_OHM => Ohm,
        /// Farad (capacity).
        SR_UNIT_FARAD => Farad,
        /// Kelvin (temperature).
        SR_UNIT_KELVIN => Kelvin,
        /// Degrees Celsius (temperature).
        SR_UNIT_CELSIUS => Celsius,
        /// Degrees Fahrenheit (temperature).
        SR_UNIT_FAHRENHEIT => Fahrenheit,
        /// Hertz (frequency, 1/s, \[Hz\]).
        SR_UNIT_HERTZ => Hertz,
        /// Percent value.
        SR_UNIT_PERCENTAGE => Percentage,
        /// Boolean value.
        SR_UNIT_BOOLEAN => Boolean,
        /// Time in seconds.
        SR_UNIT_SECOND => Second,
        /// Unit of conductance, the inverse of resistance.
        SR_UNIT_SIEMENS => Siemens,
        /// An absolute measurement of power, in decibels, referenced to 1 milliwatt (dBm).
        SR_UNIT_DECIBEL_MW => DecibelMilliWatt,
        /// Voltage in decibel, referenced to 1 volt (dBV).
        SR_UNIT_DECIBEL_VOLT => DecibelVolt,
        /// Measurements that intrinsically do not have units attached, such as ratios, gains, etc. Specifically, a transistor's gain (hFE) is a unitless quantity, for example.
        SR_UNIT_UNITLESS => Unitless,
        /// Sound pressure level, in decibels, relative to 20 micropascals.
        SR_UNIT_DECIBEL_SPL => DecibelSPL,
        /// Normalized (0 to 1) concentration of a substance or compound with 0 representing a concentration of 0%, and 1 being 100%. This is represented as the fraction of number of particles of the substance.
        SR_UNIT_CONCENTRATION => Concentration,
        /// Revolutions per minute.
        SR_UNIT_REVOLUTIONS_PER_MINUTE => RevolutionsPerMinute,
        /// Apparent power \[VA\].
        SR_UNIT_VOLT_AMPERE => VoltAmpere,
        /// Real power \[W\].
        SR_UNIT_WATT => Watt,
        /// Consumption \[Wh\].
        SR_UNIT_WATT_HOUR => WattHour,
        /// Wind speed in meters per second.
        SR_UNIT_METER_SECOND => MeterSecond,
        /// Pressure in hectopascal
        SR_UNIT_HECTOPASCAL => Hectopascal,
        /// Relative humidity assuming air temperature of 293 Kelvin (%rF).
        SR_UNIT_HUMIDITY_293K => Humidity293K,
        /// Plane angle in 1/360th of a full circle.
        SR_UNIT_DEGREE => Degree,
        /// Henry (inductance).
        SR_UNIT_HENRY => Henry,
        /// Mass in gram \[g\].
        SR_UNIT_GRAM => Gram,
        /// Mass in carat \[ct\].
        SR_UNIT_CARAT => Carat,
        /// Mass in ounce \[oz\].
        SR_UNIT_OUNCE => Ounce,
        /// Mass in troy ounce \[oz t\].
        SR_UNIT_TROY_OUNCE => TroyOunce,
        /// Mass in pound \[lb\].
        SR_UNIT_POUND => Pound,
        /// Mass in pennyweight \[dwt\].
        SR_UNIT_PENNYWEIGHT => Pennyweight,
        /// Mass in grain \[gr\].
        SR_UNIT_GRAIN => Grain,
        /// Mass in tael (variants: Hong Kong, Singapore/Malaysia, Taiwan)
        SR_UNIT_TAEL => Tael,
        /// Mass in momme.
        SR_UNIT_MOMME => Momme,
        /// Mass in tola.
        SR_UNIT_TOLA => Tola,
        /// Pieces (number of items).
        SR_UNIT_PIECE => Piece,
    }
}
define_enum! {
    #[non_exhaustive]
    pub enum MqType from sigrok_sys::sr_mq as u32 {
        SR_MQ_VOLTAGE => Voltage,
        SR_MQ_CURRENT => Current,
        SR_MQ_RESISTANCE => Resistance,
        SR_MQ_CAPACITANCE => Capacitance,
        SR_MQ_TEMPERATURE => Temperature,
        SR_MQ_FREQUENCY => Frequency,
        /// Duty cycle, e.g. on/off ratio.
        SR_MQ_DUTY_CYCLE => DutyCycle,
        /// Continuity test.
        SR_MQ_CONTINUITY => Continuity,
        SR_MQ_PULSE_WIDTH => PulseWidth,
        SR_MQ_CONDUCTANCE => Conductance,
        /// Electrical power, usually in W, or dBm.
        SR_MQ_POWER => Power,
        /// Gain (a transistor's gain, or hFE, for example).
        SR_MQ_GAIN => Gain,
        /// Logarithmic representation of sound pressure relative to a reference value.
        SR_MQ_SOUND_PRESSURE_LEVEL => SoundPressureLevel,
        /// Carbon monoxide level
        SR_MQ_CARBON_MONOXIDE => CarbonMonoxide,
        /// Humidity
        SR_MQ_RELATIVE_HUMIDITY => RelativeHumidity,
        /// Time
        SR_MQ_TIME => Time,
        /// Wind speed
        SR_MQ_WIND_SPEED => WindSpeed,
        /// Pressure
        SR_MQ_PRESSURE => Pressure,
        /// Parallel inductance (LCR meter model).
        SR_MQ_PARALLEL_INDUCTANCE => ParallelInductance,
        /// Parallel capacitance (LCR meter model).
        SR_MQ_PARALLEL_CAPACITANCE => ParallelCapacitance,
        /// Parallel resistance (LCR meter model).
        SR_MQ_PARALLEL_RESISTANCE => ParallelResistance,
        /// Series inductance (LCR meter model).
        SR_MQ_SERIES_INDUCTANCE => SeriesInductance,
        /// Series capacitance (LCR meter model).
        SR_MQ_SERIES_CAPACITANCE => SeriesCapacitance,
        /// Series resistance (LCR meter model).
        SR_MQ_SERIES_RESISTANCE => SeriesResistance,
        /// Dissipation factor.
        SR_MQ_DISSIPATION_FACTOR => DissipationFactor,
        /// Quality factor.
        SR_MQ_QUALITY_FACTOR => QualityFactor,
        /// Phase angle.
        SR_MQ_PHASE_ANGLE => PhaseAngle,
        /// Difference from reference value.
        SR_MQ_DIFFERENCE => Difference,
        /// Count.
        SR_MQ_COUNT => Count,
        /// Power factor.
        SR_MQ_POWER_FACTOR => PowerFactor,
        /// Apparent power
        SR_MQ_APPARENT_POWER => ApparentPower,
        /// Mass
        SR_MQ_MASS => Mass,
        /// Harmonic ratio
        SR_MQ_HARMONIC_RATIO => HarmonicRatio,
    }
}
define_enum! {
    /// A function of a [`Driver`][crate::Driver], [`Device`][crate::device::Device], or
    /// [`ChannelGroup`][crate::device::ChannelGroup].
    #[non_exhaustive]
    pub enum Function from sigrok_sys::sr_configkey as u32 {
        /// The device can act as logic analyzer.
        SR_CONF_LOGIC_ANALYZER => LogicAnalyzer,
        /// The device can act as an oscilloscope.
        SR_CONF_OSCILLOSCOPE => Oscilloscope,
        /// The device can act as a multimeter.
        SR_CONF_MULTIMETER => Multimeter,
        /// The device is a demo device.
        SR_CONF_DEMO_DEV => DemoDev,
        /// The device can act as a sound level meter.
        SR_CONF_SOUNDLEVELMETER => SoundLevelMeter,
        /// The device can measure temperature.
        SR_CONF_THERMOMETER => Thermometer,
        /// The device can measure humidity.
        SR_CONF_HYGROMETER => Hygrometer,
        /// The device can measure energy consumption.
        SR_CONF_ENERGYMETER => EnergyMeter,
        /// The device can act as a signal demodulator.
        SR_CONF_DEMODULATOR => Demodulator,
        /// The device can act as a programmable power supply.
        SR_CONF_POWER_SUPPLY => PowerSupply,
        /// The device can act as an LCR meter.
        SR_CONF_LCRMETER => LcrMeter,
        /// The device can act as an electronic load.
        SR_CONF_ELECTRONIC_LOAD => ElectronicLoad,
        /// The device can act as a scale.
        SR_CONF_SCALE => Scale,
        /// The device can act as a function generator.
        SR_CONF_SIGNAL_GENERATOR => SignalGenerator,
        /// The device can measure power.
        SR_CONF_POWERMETER => PowerMeter,
    }
}
define_enum! {
    /// The state to trigger on.
    ///
    /// You'll want to use this in [`Trigger`](crate::Trigger), which will be placed into
    /// [`Triggers`](crate::Triggers), and then finally [`Session::start`][crate::Session::start].
    #[non_exhaustive]
    pub enum TriggerType from sigrok_sys::sr_trigger_matches as i32 {
        SR_TRIGGER_ZERO => Zero,
        SR_TRIGGER_ONE => One,
        SR_TRIGGER_RISING => Rising,
        SR_TRIGGER_FALLING => Falling,
        SR_TRIGGER_EDGE => Edge,
        /// Used in analog channels to trigger when the voltage goes over a
        /// [specific value](crate::Trigger::value).
        SR_TRIGGER_OVER => Over,
        /// Used in analog channels to trigger when the voltage under over a
        /// [specific value](crate::Trigger::value).
        SR_TRIGGER_UNDER => Under,
    }
}
define_enum! {
    /// A log level. The default log level is [`Warn`][LogLevel::Warn].
    #[non_exhaustive]
    pub enum LogLevel from sigrok_sys::sr_loglevel as i32 {
        /// Output no messages at all.
        SR_LOG_NONE => None,
        /// Output error messages.
        SR_LOG_ERR => Err,
        /// Output warnings.
        SR_LOG_WARN => Warn,
        /// Output informational messages.
        SR_LOG_INFO => Info,
        /// Output debug messages.
        SR_LOG_DBG => Debug,
        /// Output very noisy debug messages.
        SR_LOG_SPEW => Spew,
    }
}
impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Warn
    }
}
