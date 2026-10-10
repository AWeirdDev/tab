use std::{
    ffi::{CString, c_char},
    os::unix::ffi::OsStrExt,
    path::Path,
};

use krun_sys as sys;

use crate::error_primitives::{ReferToLog, fallible};

/// VM configuration context.
pub struct Context {
    id: u32,
    deconstructors: Vec<Box<dyn std::any::Any>>,
}

impl Context {
    fn will_deconstruct<T: 'static>(&mut self, owned: T) {
        self.deconstructors.push(Box::new(owned));
    }

    pub fn new() -> Result<Self, ContextError> {
        let maybe_ctx = unsafe { sys::krun_create_ctx() };
        match fallible(maybe_ctx) {
            Some(id) => Ok(Self {
                id: id as u32,
                deconstructors: Vec::with_capacity(3),
            }),
            None => Err(ReferToLog.into()),
        }
    }

    pub fn with_vm_config(self, num_vcpus: u8, ram_mib: u32) -> Result<Self, ContextError> {
        fallible(unsafe { sys::krun_set_vm_config(self.id, num_vcpus, ram_mib) })
            .map(|_| self)
            .ok_or(ReferToLog.into())
    }

    pub fn with_root<P: AsRef<Path>>(mut self, root: P) -> Result<Self, ContextError> {
        let root = cstring_path(root)?;
        unsafe { sys::krun_set_root(self.id, root.as_ptr()) };
        self.will_deconstruct(root);
        Ok(self)
    }

    pub fn with_workdir<P: AsRef<Path>>(mut self, root: P) -> Result<Self, ContextError> {
        let dir = cstring_path(root)?;
        unsafe { sys::krun_set_workdir(self.id, dir.as_ptr()) };
        self.will_deconstruct(dir);
        Ok(self)
    }

    pub fn with_exec<P: AsRef<Path>>(
        mut self,
        executable: P,
        argv: impl Iterator<Item = impl AsRef<str>>,
        envp: impl Iterator<Item = impl AsRef<str>>,
    ) -> Result<Self, ContextError> {
        let argv_carr = CArrayOfString::new(argv)?;
        let envp_carr = CArrayOfString::new(envp)?;
        let exe = cstring_path(executable)?;

        unsafe {
            sys::krun_set_exec(
                self.id,
                exe.as_ptr(),
                argv_carr.as_ptr(),
                envp_carr.as_ptr(),
            );
        }

        self.will_deconstruct(argv_carr);
        self.will_deconstruct(envp_carr);
        self.will_deconstruct(exe);

        Ok(self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error(transparent)]
    Unknown(#[from] ReferToLog),

    #[error("the copied str has a NUL byte")]
    BadCopiedStr(#[from] std::ffi::NulError),

    #[error("the provided string has a NUL byte")]
    BadString(#[from] std::ffi::FromVecWithNulError),
}

// needs Drop
struct CArrayOfString {
    _source: Vec<CString>, // lifetime
    view: Vec<*const c_char>,
}

impl CArrayOfString {
    fn new<It: Iterator<Item = impl AsRef<str>>>(it: It) -> Result<Self, ContextError> {
        let container = it
            .map(|item| {
                Ok(castaway::match_type!(item, {
                    String as s => CString::from_vec_with_nul(s.into_bytes())?,
                    s => CString::new(s.as_ref())?
                }))
            })
            .collect::<Result<Vec<_>, ContextError>>()?;

        let mut view = Vec::with_capacity(container.len() + 1);
        for cstring in container.iter().by_ref() {
            view.push(cstring.as_ptr());
        }

        Ok(Self {
            _source: container,
            view,
        })
    }

    #[inline]
    fn as_ptr(&self) -> *const *const c_char {
        self.view.as_ptr()
    }
}

fn cstring_path<P: AsRef<Path>>(path: P) -> Result<CString, ContextError> {
    let dir = path.as_ref().as_os_str().as_bytes();
    Ok(CString::new(dir)?)
}
