// Kilometers (consistent with MU_EARTH units in km^3/s^2).
pub const EARTH_RADIUS: f32 = 6_378.136_6;
pub const J2: f64 = 1.082_63e-3;
pub const MU_EARTH: f64 = 398_600.441_8; // km^3 / s^2
pub const OMEGA_SUNSYNC_DEG_PER_DAY: f64 = 0.985_607_668_6; // required RAAN precession magnitude

// Earth rotation (Greenwich Sidereal Time) constants for ECI/ECEF alignment
pub const GMST_BASE_HOURS: f64 = 18.697_374_558;
pub const GMST_RATE_HOURS_PER_DAY: f64 = 24.065_709_824_419_08;
pub const START_DAY_OF_YEAR: f64 = 172.0;
