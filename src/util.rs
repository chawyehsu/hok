/// Convert bytes to KB/MB/GB representation.
pub fn humansize(length: u64, with_unit: bool) -> String {
    let gb: f64 = 2.0_f64.powf(30_f64);
    let mb: f64 = 2.0_f64.powf(20_f64);
    let kb: f64 = 2.0_f64.powf(10_f64);

    let flength = length as f64;

    if flength > gb {
        let j = (flength / gb).round();

        if with_unit {
            format!("{} GB", j)
        } else {
            j.to_string()
        }
    } else if flength > mb {
        let j = (flength / mb).round();

        if with_unit {
            format!("{} MB", j)
        } else {
            j.to_string()
        }
    } else if flength > kb {
        let j = (flength / kb).round();

        if with_unit {
            format!("{} KB", j)
        } else {
            j.to_string()
        }
    } else if with_unit {
        format!("{} B", flength)
    } else {
        flength.to_string()
    }
}

use windows::Win32::Foundation::BOOL;
use windows::Win32::Security::{
    CheckTokenMembership, CreateWellKnownSid, WinBuiltinAdministratorsSid, PSID,
    SECURITY_MAX_SID_SIZE,
};

pub(crate) fn is_admin() -> bool {
    // DWORD-align the backing buffer for the SID returned by CreateWellKnownSid.
    let mut sid_buffer = [0u32; (SECURITY_MAX_SID_SIZE as usize).div_ceil(size_of::<u32>())];
    let sid = PSID(sid_buffer.as_mut_ptr().cast());
    let mut sid_size = SECURITY_MAX_SID_SIZE;
    let mut is_member = BOOL(0);

    unsafe {
        if CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            PSID::default(),
            sid,
            &mut sid_size,
        )
        .is_err()
        {
            return false;
        }

        // A null token handle checks the calling thread's effective token, falling back to the
        // process token when the thread is not impersonating.
        if CheckTokenMembership(
            windows::Win32::Foundation::HANDLE::default(),
            sid,
            &mut is_member,
        )
        .is_err()
        {
            return false;
        }
    }

    is_member.as_bool()
}
