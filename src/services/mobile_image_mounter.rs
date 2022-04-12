// jkcoxson

use std::io::Read;

use libc::c_void;
use plist_plus::Plist;

use super::lockdownd::LockdowndService;
use crate::{bindings as unsafe_bindings, debug, error::MobileImageMounterError};

pub struct MobileImageMounter<'a> {
    pub(crate) pointer: unsafe_bindings::mobile_image_mounter_client_t,
    pub(crate) phantom: std::marker::PhantomData<&'a LockdowndService<'a>>,
}

unsafe impl Send for MobileImageMounter<'_> {}
unsafe impl Sync for MobileImageMounter<'_> {}

#[cfg(not(target_os = "windows"))]
type ImageMounterPointerSize = u64;
#[cfg(not(target_os = "windows"))]
type ImageMounterReturnType = i64;

impl MobileImageMounter<'_> {
    /// Uploads an image from a path to the device
    pub fn upload_image(
        &self,
        image_path: String,
        image_type: String,
        signature_path: String,
    ) -> Result<(), MobileImageMounterError> {
        // Determine if files exist
        let dmg_size = match std::fs::File::open(image_path.clone()) {
            Ok(mut file) => {
                let mut temp_buf = vec![];
                file.read_to_end(&mut temp_buf).unwrap();
                temp_buf.len()
            }
            Err(_) => return Err(MobileImageMounterError::DmgNotFound),
        };
        let signature_size = match std::fs::File::open(signature_path.clone()) {
            Ok(mut file) => {
                let mut temp_buf = vec![];
                file.read_to_end(&mut temp_buf).unwrap();
                temp_buf.len()
            }
            Err(_) => return Err(MobileImageMounterError::SignatureNotFound),
        };
        // Read the image into a buffer
        let image_path_c_str = &mut std::ffi::CString::new(image_path.clone()).unwrap();
        let mode_c_str = &mut std::ffi::CString::new("rb").unwrap();
        debug!("Opening image file");
        let image_buffer = unsafe { libc::fopen(image_path_c_str.as_ptr(), mode_c_str.as_ptr()) };
        // Read the signature into a buffer
        let signature_path_c_str = &mut std::ffi::CString::new(signature_path.clone()).unwrap();
        debug!("Reading signature file");
        let signature_buffer =
            unsafe { libc::fopen(signature_path_c_str.as_ptr(), mode_c_str.as_ptr()) };

        let image_type_c_str = std::ffi::CString::new(image_type.clone()).unwrap();
        let image_type_c_str = if image_type == "".to_string() {
            std::ptr::null()
        } else {
            image_type_c_str.as_ptr()
        };

        debug!("Uploading image");
        let result = unsafe {
            unsafe_bindings::mobile_image_mounter_upload_image(
                self.pointer,
                image_type_c_str,
                dmg_size as ImageMounterPointerSize,
                signature_buffer as *const i8,
                signature_size as u16,
                Some(image_mounter_callback),
                image_buffer as *mut c_void,
            )
        }
        .into();

        if result != MobileImageMounterError::Success {
            return Err(result);
        }

        Ok(())
    }

    /// Mounts the image on the device
    pub fn mount_image(
        &self,
        image_path: String,
        image_type: String,
        signature_path: String,
    ) -> Result<Plist, MobileImageMounterError> {
        // Read the image into a buffer
        let mut image_buffer = Vec::new();
        let file = match std::fs::File::open(image_path.clone()) {
            Ok(file) => file,
            Err(_) => return Err(MobileImageMounterError::DmgNotFound),
        };
        let mut reader = std::io::BufReader::new(file);
        match reader.read_to_end(&mut image_buffer) {
            Ok(_) => (),
            Err(_) => return Err(MobileImageMounterError::DmgNotFound),
        };
        // Read the signature into a buffer
        let mut signature_buffer = Vec::new();
        let file = match std::fs::File::open(signature_path) {
            Ok(file) => file,
            Err(_) => return Err(MobileImageMounterError::SignatureNotFound),
        };
        let mut reader = std::io::BufReader::new(file);
        match reader.read_to_end(&mut signature_buffer) {
            Ok(_) => (),
            Err(_) => return Err(MobileImageMounterError::SignatureNotFound),
        };
        let image_type_c_str = std::ffi::CString::new(image_type.clone()).unwrap();
        let image_type_c_str = if image_type == "".to_string() {
            std::ptr::null()
        } else {
            image_type_c_str.as_ptr()
        };

        let mut plist: unsafe_bindings::plist_t = unsafe { std::mem::zeroed() };

        debug!("Mounting image");
        let result = unsafe {
            unsafe_bindings::mobile_image_mounter_mount_image(
                self.pointer,
                image_path.as_ptr() as *const i8,
                signature_buffer.as_ptr() as *const i8,
                signature_buffer.len() as u16,
                image_type_c_str,
                &mut plist,
            )
        }
        .into();

        if result != MobileImageMounterError::Success {
            return Err(result);
        }
        Ok(plist.into())
    }

    pub fn lookup_image(&self, image_type: String) -> Result<Plist, MobileImageMounterError> {
        let image_type_c_str = std::ffi::CString::new(image_type.clone()).unwrap();
        let image_type_c_str = if image_type == "".to_string() {
            std::ptr::null()
        } else {
            image_type_c_str.as_ptr()
        };

        let mut plist: unsafe_bindings::plist_t = unsafe { std::mem::zeroed() };

        debug!("Looking up image");
        let result = unsafe {
            unsafe_bindings::mobile_image_mounter_lookup_image(
                self.pointer,
                image_type_c_str,
                &mut plist,
            )
        }
        .into();

        if result != MobileImageMounterError::Success {
            return Err(result);
        }
        Ok(plist.into())
    }
}

extern "C" fn image_mounter_callback(
    a: *mut c_void,
    b: ImageMounterPointerSize,
    c: *mut c_void,
) -> ImageMounterReturnType {
    debug!("image_mounter_callback called");
    return unsafe { libc::fread(a, 1, b as usize, c as *mut libc::FILE) }
        as ImageMounterReturnType;
}

impl Drop for MobileImageMounter<'_> {
    fn drop(&mut self) {
        debug!("Dropping MobileImageMounter");
        unsafe {
            unsafe_bindings::mobile_image_mounter_free(self.pointer);
        }
    }
}