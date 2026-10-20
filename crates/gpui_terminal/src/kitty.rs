//! Presents rendered frames with the kitty graphics protocol.
//!
//! Each frame is a POSIX shared memory object (`t=s`) that the GPU readback
//! writes into directly, as RGBA, so the pty only carries a ~100 byte escape
//! code and the CPU copies each pixel once. The terminal maps the object,
//! reads it, then unlinks it. Re-transmitting image id 1 with placement id 1
//! (`a=T,i=1,p=1`) replaces the old frame and its placement in one command,
//! and the synchronized update brackets (`CSI ? 2026`) keep it flicker free.
//! https://sw.kovidgoyal.net/kitty/graphics-protocol/

use std::{collections::VecDeque, ffi::CString, fmt::Write as _, io, ptr};

use anyhow::{Context as _, Result};
use base64::Engine as _;

use crate::tty::Tty;

/// Frames whose shared memory object we still own. The terminal unlinks an
/// object once it read it; we unlink anything older than this ourselves, so a
/// terminal that stops reading cannot leak memory without bound.
const IN_FLIGHT_FRAMES: usize = 4;

pub(crate) struct KittyPresenter {
    process_id: u32,
    frame_counter: u64,
    in_flight: VecDeque<CString>,
    command: String,
}

/// A mapped shared memory object the next frame is rendered into.
pub(crate) struct Frame {
    name: CString,
    mapping: *mut libc::c_void,
    length: usize,
    width: u32,
    height: u32,
    presented: bool,
}

impl Frame {
    pub(crate) fn pixels(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.mapping as *mut u8, self.length) }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.mapping, self.length) };
        if !self.presented {
            unsafe { libc::shm_unlink(self.name.as_ptr()) };
        }
    }
}

impl KittyPresenter {
    pub(crate) fn new() -> Self {
        Self {
            process_id: std::process::id(),
            frame_counter: 0,
            in_flight: VecDeque::new(),
            command: String::new(),
        }
    }

    /// Creates the shared memory object for an RGBA frame of `width` x `height`.
    /// Dropping the frame without `present` discards it.
    pub(crate) fn begin_frame(&mut self, width: u32, height: u32) -> Result<Frame> {
        // macOS caps shared memory names at 31 bytes including the slash.
        let name = format!("/gpui{:x}.{:x}", self.process_id, self.frame_counter);
        self.frame_counter += 1;
        let name = CString::new(name).context("shared memory name")?;
        let length = width as usize * height as usize * 4;
        let mapping = map_shared_memory(&name, length)?;
        Ok(Frame {
            name,
            mapping,
            length,
            width,
            height,
            presented: false,
        })
    }

    /// Shows `frame` covering the grid from its top-left cell.
    pub(crate) fn present(&mut self, tty: &mut Tty, mut frame: Frame) -> Result<()> {
        if self.in_flight.len() >= IN_FLIGHT_FRAMES
            && let Some(stale) = self.in_flight.pop_front()
        {
            // ENOENT is the normal case: the terminal already unlinked it.
            unsafe { libc::shm_unlink(stale.as_ptr()) };
        }

        let encoded_name = base64::engine::general_purpose::STANDARD.encode(frame.name.as_bytes());
        self.command.clear();
        write!(
            self.command,
            "\x1b[?2026h\x1b[H\x1b_Ga=T,i=1,p=1,f=32,t=s,s={},v={},S={},C=1,q=2;{encoded_name}\x1b\\\x1b[?2026l",
            frame.width, frame.height, frame.length,
        )?;
        frame.presented = true;
        self.in_flight.push_back(frame.name.clone());
        drop(frame);
        tty.write_all(self.command.as_bytes())
    }
}

impl Drop for KittyPresenter {
    fn drop(&mut self) {
        for name in self.in_flight.drain(..) {
            unsafe { libc::shm_unlink(name.as_ptr()) };
        }
    }
}

/// macOS refuses write(2) on shared memory, so the object is always mapped.
fn map_shared_memory(name: &CString, length: usize) -> Result<*mut libc::c_void> {
    let fd = unsafe {
        libc::shm_open(
            name.as_ptr(),
            libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
            0o600 as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error()).context("shm_open");
    }
    let result = if unsafe { libc::ftruncate(fd, length as libc::off_t) } != 0 {
        Err(io::Error::last_os_error()).context("ftruncate")
    } else {
        let mapping = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if mapping == libc::MAP_FAILED {
            Err(io::Error::last_os_error()).context("mmap")
        } else {
            Ok(mapping)
        }
    };
    unsafe { libc::close(fd) };
    if result.is_err() {
        unsafe { libc::shm_unlink(name.as_ptr()) };
    }
    result
}
