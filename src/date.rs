use core::fmt;

#[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::January => "January",
            Self::February => "February",
            Self::March => "March",
            Self::April => "April",
            Self::May => "May",
            Self::June => "June",
            Self::July => "July",
            Self::August => "August",
            Self::September => "September",
            Self::October => "October",
            Self::November => "November",
            Self::December => "December",
        })
    }
}

impl TryFrom<u8> for Month {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let month = match value {
            0 => Self::January,
            1 => Self::February,
            2 => Self::March,
            3 => Self::April,
            4 => Self::May,
            5 => Self::June,
            6 => Self::July,
            7 => Self::August,
            8 => Self::September,
            9 => Self::October,
            10 => Self::November,
            11 => Self::December,
            _ => return Err(()),
        };
        Ok(month)
    }
}

pub const SECONDS_PER_MINUTE: u32 = 60;
pub const SECONDS_PER_HOUR: u32 = 60 * SECONDS_PER_MINUTE;
pub const SECONDS_PER_DAY: u32 = 24 * SECONDS_PER_HOUR;

const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const DAYS_IN_MONTH_LEAP_YEAR: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

pub struct Date {
    second: u8,
    minute: u8,
    hour: u8,
    month: Month,
    day_of_month: u8,
    day_of_year: u16,
    year: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    AM,
    PM,
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AM => "AM",
            Self::PM => "PM",
        })
    }
}

impl Date {
    pub fn from_utime(mut utime: i64) -> Self {
        let second: u8;
        let minute: u8;
        let hour: u8;
        let mut month: u8 = 0;
        let day_of_month: u8;
        let day_of_year: u16;
        let mut year: i32 = 1970;
        let mut leap_year = false;

        if utime >= 0 {
            loop {
                let days = if leap_year { 366 } else { 365 };
                let needed = (SECONDS_PER_DAY as i32 * days) as i64;
                if utime < needed {
                    break;
                }
                utime -= needed;
                year += 1;
                leap_year = is_leap_year(year);
            }
        } else {
            while utime < 0 {
                year -= 1;
                leap_year = is_leap_year(year);
                let days = if leap_year { 366 } else { 365 };
                let needed = (SECONDS_PER_DAY as i32 * days) as i64;
                if utime + needed < 0 {
                    // NOTE: Normalize the utime to a positive complement.
                    utime += needed;
                    break;
                }
                utime += needed;
            }
        }

        let mut utime: u64 = utime as u64;
        day_of_year = (utime / SECONDS_PER_DAY as u64).try_into().unwrap();
        debug_assert!(day_of_year < 366);

        let days_in_month = if leap_year {
            &DAYS_IN_MONTH_LEAP_YEAR
        } else {
            &DAYS_IN_MONTH
        };

        loop {
            let needed = days_in_month[month as usize] as u32 * SECONDS_PER_DAY;
            if utime < needed as u64 {
                break;
            }
            utime -= needed as u64;
            month += 1;
        }
        debug_assert!(month < 12);
        let month = Month::try_from(month).unwrap();

        day_of_month = (utime / SECONDS_PER_DAY as u64).try_into().unwrap();
        debug_assert!(day_of_month < 31);

        utime %= SECONDS_PER_DAY as u64;

        hour = (utime / SECONDS_PER_HOUR as u64).try_into().unwrap();
        debug_assert!(hour < 24);

        utime %= SECONDS_PER_HOUR as u64;

        minute = (utime / SECONDS_PER_MINUTE as u64).try_into().unwrap();
        debug_assert!(minute < 60);

        utime %= SECONDS_PER_MINUTE as u64;
        second = utime.try_into().unwrap();
        debug_assert!(second < 60);

        Self {
            second,
            minute,
            hour,
            month,
            day_of_month,
            day_of_year,
            year,
        }
    }

    pub fn second(&self) -> u8 {
        self.second
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn hour_12(&self) -> u8 {
        match self.hour % 12 {
            0 => 12u8,
            x => x,
        }
    }

    pub fn period(&self) -> Period {
        if self.hour < 12 {
            Period::AM
        } else {
            Period::PM
        }
    }

    pub fn month(&self) -> Month {
        self.month
    }

    pub fn day_of_month(&self) -> u8 {
        self.day_of_month + 1
    }

    pub fn day_of_year(&self) -> u16 {
        self.day_of_year + 1
    }

    pub fn year(&self) -> i32 {
        self.year
    }
}

pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
