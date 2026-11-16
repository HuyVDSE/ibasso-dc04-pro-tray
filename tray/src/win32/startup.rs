use std::os::windows::ffi::OsStrExt;
use windows::{
    Win32::{Foundation::*, System::Registry::*},
    core::{Error, PCWSTR, Result, w},
};

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const NAME: PCWSTR = w!("IbassoDc04ProTray");
// Run supports commands of at most 260 characters, including the quotes.
const MAX_COMMAND: usize = 260;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Off,
    Current,
    Other,
}

pub struct Startup {
    command: Vec<u16>,
}

impl Startup {
    pub fn current_exe() -> Result<Self> {
        let path =
            std::env::current_exe().map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        let command = [u16::from(b'"')]
            .into_iter()
            .chain(path.as_os_str().encode_wide())
            .chain([u16::from(b'"'), 0])
            .collect();
        Ok(Self { command })
    }

    pub fn state(&self) -> Result<State> {
        let mut data = [0_u16; MAX_COMMAND + 1];
        let mut size = size_of_val(&data) as u32;
        let mut kind = REG_VALUE_TYPE::default();
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                RUN,
                NAME,
                RRF_RT_ANY | RRF_NOEXPAND,
                Some(&mut kind),
                Some(data.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        match result {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(State::Off),
            ERROR_MORE_DATA => Ok(State::Other),
            ERROR_SUCCESS => {
                let size = size as usize;
                let current = kind == REG_SZ
                    && size == size_of_val(self.command.as_slice())
                    && data.get(..size / 2) == Some(self.command.as_slice());
                Ok(if current {
                    State::Current
                } else {
                    State::Other
                })
            }
            error => Err(Error::from(error.to_hresult())),
        }
    }

    pub fn enable(&self) -> Result<()> {
        if self.command.len() > MAX_COMMAND + 1 {
            return Err(Error::new(
                E_INVALIDARG,
                "Đường dẫn quá dài để đăng ký. Hãy chuyển app sang thư mục có đường dẫn ngắn hơn.",
            ));
        }
        let mut key = Key(HKEY::default());
        unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                RUN,
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key.0,
                None,
            )
            .ok()?;
            let bytes = std::slice::from_raw_parts(
                self.command.as_ptr().cast::<u8>(),
                size_of_val(self.command.as_slice()),
            );
            RegSetValueExW(key.0, NAME, None, REG_SZ, Some(bytes)).ok()
        }
    }

    pub fn disable(&self) -> Result<()> {
        let mut key = Key(HKEY::default());
        unsafe {
            let result = RegOpenKeyExW(HKEY_CURRENT_USER, RUN, None, KEY_SET_VALUE, &mut key.0);
            if result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND {
                return Ok(());
            }
            result.ok()?;
            match RegDeleteValueW(key.0, NAME) {
                ERROR_FILE_NOT_FOUND => Ok(()),
                result => result.ok(),
            }
        }
    }
}

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
}
