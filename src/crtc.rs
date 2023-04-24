use crate::Xid;
use crate::Time;
use crate::CURRENT_TIME;
use crate::XHandle;
use crate::XrandrError;
use std::ptr;
use std::slice;

use x11::xrandr;
use std::convert::TryFrom;

#[derive(Copy, Debug, Clone)]
pub enum Rotation {
    Normal = 1,
    Left = 2,
    Inverted = 4,
    Right = 8,
}

impl TryFrom<u16> for Rotation {
    type Error = XrandrError;

    fn try_from(r: u16) -> Result<Self, Self::Error> {
        match r {
            1 => Ok(Rotation::Normal),
            2 => Ok(Rotation::Left),
            4 => Ok(Rotation::Inverted),
            8 => Ok(Rotation::Right),
            _ => Err(XrandrError::InvalidRotation(r)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Crtc {
    pub xid: Xid,
    pub timestamp: Time,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub mode: Xid,
    pub rotation: Rotation,
    pub outputs: Vec<Xid>,
    pub rotations: u16,
    pub possible: Vec<Xid>,
}


/// Normalizes a set of Crtcs by making sure the top left pixel of the screen
/// is at (0,0). This is needed after changing positions/rotations.
pub(crate) fn normalize_positions(crtcs: &[Crtc]) -> Vec<Crtc> {
    assert!(!crtcs.is_empty());
    let left = crtcs.iter().map(|p| p.x).min().unwrap();
    let top = crtcs.iter().map(|p| p.y).min().unwrap();

    return crtcs.iter()
        .map(|p| p.offset((-left, -top)))
        .collect();
}


impl Crtc {
    // TODO: better error documentation
    /// Open a handle to the lib-xrandr backend. This will be 
    /// used for nearly all interactions with the xrandr lib
    ///
    /// # Arguments
    /// * `handle` - The xhandle to make the x calls with
    /// * `xid` - The internal XID of the requested crtc
    ///
    /// # Errors
    /// * `XrandrError::GetCrtc(xid)` - Could not find this xid.
    ///
    /// # Examples
    /// ```
    /// let xhandle = XHandle.open()?;
    /// let mon1 = xhandle.monitors()?[0];
    /// ```
    ///
    pub fn from_xid(handle: &mut XHandle, xid: Xid) 
    -> Result<Self,XrandrError>
    {
        let c_i = unsafe {
            ptr::NonNull::new(xrandr::XRRGetCrtcInfo(
                handle.sys.as_ptr(),
                handle.res()?,
                xid,
            ))
            .ok_or(XrandrError::GetCrtc(xid))?
            .as_mut()
        };


        let rotation = Rotation::try_from(c_i.rotation)?;

        let outputs = unsafe { 
            slice::from_raw_parts(c_i.outputs, c_i.noutput as usize) 
        }.to_vec();

        let possible = unsafe { 
            slice::from_raw_parts(c_i.possible, c_i.npossible as usize) 
        }.to_vec();

        let result = Self {
            xid,
            timestamp: c_i.timestamp,
            x: c_i.x,
            y: c_i.y,
            width: c_i.width,
            height: c_i.height,
            mode: c_i.mode,
            rotation,
            outputs,
            rotations: c_i.rotations,
            possible,
        };
        
        unsafe { xrandr::XRRFreeCrtcInfo(c_i as *const _ as *mut _) };
        Ok(result)
    }

    pub(crate) fn apply(&mut self, handle: &mut XHandle) 
    -> Result<(), XrandrError> 
    {
        unsafe {
            xrandr::XRRSetCrtcConfig(
                handle.sys.as_ptr(),
                handle.res()?,
                self.xid,
                CURRENT_TIME,
                self.x,
                self.y,
                self.mode,
                self.rotation as u16,
                self.outputs.as_mut_ptr(),
                self.outputs.len() as i32,
            );
        }

        Ok(())
    }

    pub(crate) fn disable(&self, handle: &mut XHandle) 
    -> Result<(), XrandrError> 
    {
        unsafe {
            xrandr::XRRSetCrtcConfig(
                handle.sys.as_ptr(),
                handle.res()?,
                self.xid, 
                CURRENT_TIME,
                0, 0, 0, Rotation::Normal as u16,
                std::ptr::null_mut(), 0);
        }

        Ok(())
    }

    // Width and height, accounting for rotation
    pub fn rot_size(&self, rot: Rotation) -> (u32, u32) {
        let (w, h) = (self.width, self.height);

        let (old_w, old_h) = match self.rotation {
            Rotation::Normal | Rotation::Inverted   => (w, h),
            Rotation::Left | Rotation::Right        => (h, w),
        };

        let x = match rot {
            Rotation::Normal | Rotation::Inverted   => (old_w, old_h),
            Rotation::Left | Rotation::Right        => (old_h, old_w),
        };

        eprintln!("Rot size: ({:?}) = {}x{}", self.rotation, x.0, x.1);
        x
    }

    pub(crate) fn max_coordinates(&self) -> (u32, u32) {
        assert!(self.x >= 0 && self.y >= 0); // Must be normalized
        // let (w, h) = self.rot_size();
        // I think crtcs have this incorporated in their width/height fields
        (self.x as u32 + self.width, self.y as u32 + self.height)
    }

    pub(crate) fn offset(&self, offset: (i32, i32)) -> Self {
        let x = self.x as i64 + offset.0 as i64;
        let y = self.y as i64 + offset.1 as i64;

        assert!(x < i32::MAX as i64 && y < i32::MAX as i64);
        // This should hold after offsetting (normalized)
        assert!(x >= 0 && y >= 0);

        let mut new = self.clone();
        new.x = x as i32;
        new.y = y as i32;
        new
    }
}
