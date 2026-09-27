//! One-shot, local-only named pipe restricted to the launching user's SID.
use anyhow::Result;
use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    time::{Duration, Instant},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{
            CloseHandle, LocalFree, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING,
            HANDLE, HLOCAL, INVALID_HANDLE_VALUE, WIN32_ERROR,
        },
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SDDL_REVISION_1,
            },
            GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
            TOKEN_USER,
        },
        Storage::FileSystem::{
            CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_FIRST_PIPE_INSTANCE,
            FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_MODE, OPEN_EXISTING,
            PIPE_ACCESS_DUPLEX,
        },
        System::{
            Pipes::{
                ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId,
                GetNamedPipeServerProcessId, PeekNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
                PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
            },
            Threading::{
                GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
                PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    },
};

const MAX_MESSAGE: usize = 4096;
pub struct NamedPipe {
    handle: HANDLE,
    buffer: Vec<u8>,
}

pub fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

fn user_security_descriptor() -> Result<PSECURITY_DESCRIPTOR> {
    let mut token = HANDLE::default();
    // SAFETY: output handles and aligned token buffer remain valid throughout each call.
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let mut needed = 0;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buffer = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
        let result = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        );
        let _ = CloseHandle(token);
        result?;
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut sid = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut sid)?;
        let sid_text = sid.to_string();
        let _ = LocalFree(Some(HLOCAL(sid.0.cast())));
        let sddl = wide(OsStr::new(&format!(
            "D:P(A;;GA;;;SY)(A;;GA;;;{})",
            sid_text?
        )));
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )?;
        Ok(descriptor)
    }
}

impl NamedPipe {
    pub fn connect(name: &str) -> Result<Self> {
        let name = wide(OsStr::new(name));
        // SAFETY: name is a live null-terminated string; the returned handle is owned here.
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
        Ok(Self {
            handle,
            buffer: Vec::new(),
        })
    }

    pub fn create_server(name: &str) -> Result<Self> {
        let name = wide(OsStr::new(name));
        let descriptor = user_security_descriptor()?;
        let security = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        // SAFETY: the descriptor and name outlive this call; Windows copies the descriptor.
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                MAX_MESSAGE as u32,
                MAX_MESSAGE as u32,
                0,
                Some(&security),
            )
        };
        let error = windows::core::Error::from_thread();
        unsafe {
            let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        }
        anyhow::ensure!(
            handle != INVALID_HANDLE_VALUE,
            "Cannot create recovery pipe: {error}"
        );
        Ok(Self {
            handle,
            buffer: Vec::new(),
        })
    }

    pub fn wait_for_client(&self, timeout: Duration, cancelled: impl Fn() -> bool) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < timeout && !cancelled() {
            // SAFETY: handle is owned and valid; nonblocking pipe avoids indefinite wait.
            match unsafe { ConnectNamedPipe(self.handle, None) } {
                Ok(()) => return Ok(()),
                Err(error) if WIN32_ERROR::from_error(&error) == Some(ERROR_PIPE_CONNECTED) => {
                    return Ok(())
                }
                Err(error) if WIN32_ERROR::from_error(&error) == Some(ERROR_PIPE_LISTENING) => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                Err(error) => return Err(error.into()),
            }
        }
        anyhow::bail!("Recovery connection cancelled or timed out")
    }

    pub fn verify_peer(&self, expected_pid: u32, server: bool) -> Result<()> {
        let mut pid = 0;
        // SAFETY: both APIs write a u32 to the supplied valid output pointer.
        unsafe {
            if server {
                GetNamedPipeServerProcessId(self.handle, &mut pid)?;
            } else {
                GetNamedPipeClientProcessId(self.handle, &mut pid)?;
            }
        }
        anyhow::ensure!(pid == expected_pid, "Unexpected recovery process");
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)? };
        let mut path = [0u16; 32768];
        let mut size = path.len() as u32;
        let result = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(path.as_mut_ptr()),
                &mut size,
            )
        };
        unsafe {
            let _ = CloseHandle(handle);
        }
        result?;
        let peer_path = String::from_utf16_lossy(&path[..size as usize]);
        anyhow::ensure!(
            std::path::Path::new(&peer_path) == std::env::current_exe()?,
            "Recovery peer executable mismatch"
        );
        Ok(())
    }

    pub fn read_line(&mut self, timeout: Duration, cancelled: impl Fn() -> bool) -> Result<String> {
        let start = Instant::now();
        while start.elapsed() < timeout && !cancelled() {
            if let Some(pos) = self.buffer.iter().position(|b| *b == b'\n') {
                return Ok(String::from_utf8(self.buffer.drain(..=pos).collect())?
                    .trim_end()
                    .to_string());
            }
            anyhow::ensure!(
                self.buffer.len() < MAX_MESSAGE,
                "Recovery message too large"
            );
            let mut available = 0;
            unsafe {
                PeekNamedPipe(self.handle, None, 0, None, Some(&mut available), None)?;
            }
            if available == 0 {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            let mut chunk = [0u8; MAX_MESSAGE];
            let mut read = 0;
            match unsafe { ReadFile(self.handle, Some(&mut chunk), Some(&mut read), None) } {
                Ok(()) => self.buffer.extend_from_slice(&chunk[..read as usize]),
                Err(error) if WIN32_ERROR::from_error(&error) == Some(ERROR_BROKEN_PIPE) => {
                    anyhow::bail!("Recovery process disconnected")
                }
                Err(error) => return Err(error.into()),
            }
            anyhow::ensure!(
                self.buffer.len() <= MAX_MESSAGE,
                "Recovery message too large"
            );
        }
        anyhow::bail!("Recovery response cancelled or timed out")
    }

    pub fn write_line(&self, line: &str) -> Result<()> {
        anyhow::ensure!(line.len() < MAX_MESSAGE, "Recovery message too large");
        let mut payload = line.as_bytes().to_vec();
        payload.push(b'\n');
        let mut written = 0;
        unsafe {
            WriteFile(self.handle, Some(&payload), Some(&mut written), None)?;
        }
        anyhow::ensure!(
            written as usize == payload.len(),
            "Incomplete recovery message"
        );
        Ok(())
    }
}
impl Drop for NamedPipe {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pipe_authenticates_round_trip_and_rejects_wrong_pid() -> Result<()> {
        let name = format!(r"\\.\pipe\gear_vr_test_{:?}", windows::core::GUID::new()?);
        let mut server = NamedPipe::create_server(&name)?;
        let client_name = name.clone();
        let client = std::thread::spawn(move || -> Result<()> {
            let mut client = NamedPipe::connect(&client_name)?;
            client.verify_peer(std::process::id(), true)?;
            client.write_line("ping")?;
            assert_eq!(client.read_line(Duration::from_secs(2), || false)?, "pong");
            Ok(())
        });
        server.wait_for_client(Duration::from_secs(2), || false)?;
        server.verify_peer(std::process::id(), false)?;
        assert!(server.verify_peer(0, false).is_err());
        assert_eq!(server.read_line(Duration::from_secs(2), || false)?, "ping");
        server.write_line("pong")?;
        client
            .join()
            .map_err(|_| anyhow::anyhow!("IPC test thread panicked"))??;
        Ok(())
    }
    #[test]
    fn absent_client_times_out_and_large_messages_are_rejected() -> Result<()> {
        let name = format!(
            r"\\.\pipe\gear_vr_timeout_{:?}",
            windows::core::GUID::new()?
        );
        let server = NamedPipe::create_server(&name)?;
        assert!(server
            .wait_for_client(Duration::from_millis(30), || false)
            .is_err());
        assert!(server.write_line(&"x".repeat(MAX_MESSAGE)).is_err());
        Ok(())
    }
}
