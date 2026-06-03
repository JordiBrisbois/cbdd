#[cfg(windows)]
pub fn dpapi_protect(input: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let description: Vec<u16> = "CRVI Backup"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let ok = unsafe {
        CryptProtectData(
            &mut input_blob,
            description.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            0,
            &mut output_blob,
        )
    };
    if ok == 0 {
        return Err("Windows n'a pas pu chiffrer le backup local.".into());
    }

    let output = unsafe {
        let slice = std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize);
        let bytes = slice.to_vec();
        LocalFree(output_blob.pbData.cast());
        bytes
    };
    Ok(output)
}

#[cfg(not(windows))]
pub fn dpapi_protect(_input: &[u8]) -> Result<Vec<u8>, String> {
    Err("Le chiffrement DPAPI n'est disponible que sous Windows.".into())
}

#[cfg(windows)]
pub fn dpapi_unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let mut description_ptr: *mut u16 = null_mut();

    let ok = unsafe {
        CryptUnprotectData(
            &mut input_blob,
            &mut description_ptr,
            null(),
            null_mut(),
            null_mut(),
            0,
            &mut output_blob,
        )
    };
    if ok == 0 {
        return Err("Windows n'a pas pu déchiffrer le backup local.".into());
    }

    let output = unsafe {
        let slice = std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize);
        let bytes = slice.to_vec();
        LocalFree(output_blob.pbData.cast());
        if !description_ptr.is_null() {
            LocalFree(description_ptr.cast());
        }
        bytes
    };
    Ok(output)
}

#[cfg(not(windows))]
pub fn dpapi_unprotect(_input: &[u8]) -> Result<Vec<u8>, String> {
    Err("Le déchiffrement DPAPI n'est disponible que sous Windows.".into())
}
