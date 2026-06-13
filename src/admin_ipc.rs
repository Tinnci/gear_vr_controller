use anyhow::Result;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{Error as WinError, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_SHARE_MODE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};

pub const PIPE_NAME: &str = r"\\.\pipe\gear_vr_admin_worker";

const PIPE_BUFFER_SIZE: u32 = 4096;

pub struct NamedPipe {
    handle: HANDLE,
    read_buffer: Vec<u8>,
}

impl NamedPipe {
    pub fn connect(name: &str) -> Result<Self> {
        let name = wide_string(name);
        let handle = unsafe {
            CreateFileW(
                PCWSTR(name.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        };

        Ok(Self::new(handle))
    }

    pub fn create_server(name: &str) -> Result<Self> {
        let name = wide_string(name);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                PIPE_BUFFER_SIZE,
                PIPE_BUFFER_SIZE,
                0,
                None,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(WinError::from_thread().into());
        }

        Ok(Self::new(handle))
    }

    pub fn wait_for_client(&self) -> Result<()> {
        match unsafe { ConnectNamedPipe(self.handle, None) } {
            Ok(()) => Ok(()),
            Err(e) if WIN32_ERROR::from_error(&e) == Some(ERROR_PIPE_CONNECTED) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn read_line(&mut self) -> Result<Option<String>> {
        loop {
            if let Some(pos) = self.read_buffer.iter().position(|&byte| byte == b'\n') {
                let line = self.read_buffer.drain(..=pos).collect::<Vec<_>>();
                return Ok(Some(String::from_utf8_lossy(&line).trim_end().to_string()));
            }

            let mut chunk = [0u8; PIPE_BUFFER_SIZE as usize];
            let mut bytes_read = 0u32;

            match unsafe { ReadFile(self.handle, Some(&mut chunk), Some(&mut bytes_read), None) } {
                Ok(()) if bytes_read == 0 => return Ok(None),
                Ok(()) => self
                    .read_buffer
                    .extend_from_slice(&chunk[..bytes_read as usize]),
                Err(e) if WIN32_ERROR::from_error(&e) == Some(ERROR_BROKEN_PIPE) => {
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub fn write_line(&mut self, line: &str) -> Result<()> {
        let mut payload = line.as_bytes().to_vec();
        payload.push(b'\n');

        let mut written_total = 0usize;
        while written_total < payload.len() {
            let mut bytes_written = 0u32;
            unsafe {
                WriteFile(
                    self.handle,
                    Some(&payload[written_total..]),
                    Some(&mut bytes_written),
                    None,
                )?;
            }
            if bytes_written == 0 {
                anyhow::bail!("Named pipe write returned zero bytes");
            }
            written_total += bytes_written as usize;
        }

        Ok(())
    }

    pub fn disconnect(&self) {
        let _ = unsafe { DisconnectNamedPipe(self.handle) };
    }

    fn new(handle: HANDLE) -> Self {
        Self {
            handle,
            read_buffer: Vec::new(),
        }
    }
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

fn wide_string(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
