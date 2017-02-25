#![feature(box_syntax)]

#[macro_use]
extern crate objc;
extern crate gleam;
extern crate cocoa;
extern crate objc_foundation;
extern crate libc;
extern crate core_foundation;
extern crate cgl;

use cocoa::appkit::*;
use cocoa::base::*;
use cocoa::foundation::*;

use cgl::{CGLEnable, kCGLCECrashOnRemovedFunctions};

use std::env::args;
use std::str::FromStr;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};
use std::os::raw::c_void;

mod app;
mod app_delegate;
mod servo;

use servo::{FollowLinkPolicy, Servo};

#[derive(Copy, Clone)]
pub struct DrawableGeometry {
    inner_size: (u32, u32),
    position: (i32, i32),
    hidpi_factor: f32,
}

// FIXME: this should hold a reference to the gl context. As
// of now, it's stored in the app delegate
// Used by Servo to wake up the event loop
pub struct EventLoopRiser {
}

impl EventLoopRiser {
    pub fn rise(&self) {
        println!("riser");
        unsafe {
            let app: id = NSApp();
            let delegate: id = msg_send![app, delegate];
            msg_send![delegate, performSelectorOnMainThread:sel!(flushGlContext) withObject:nil waitUntilDone:NO];
        }
    }
    pub fn clone(&self) -> EventLoopRiser {
        EventLoopRiser {}
    }
}


fn main() {
    let (app, glview) = app::load_nib();


    let cxt = unsafe {

        glview.setWantsBestResolutionOpenGLSurface_(YES);

        let attributes = vec![
            NSOpenGLPFADoubleBuffer as u32,
            NSOpenGLPFAClosestPolicy as u32,
            NSOpenGLPFAColorSize as u32,
            32,
            NSOpenGLPFAAlphaSize as u32,
            8,
            NSOpenGLPFADepthSize as u32,
            24,
            NSOpenGLPFAStencilSize as u32,
            8,
            NSOpenGLPFAOpenGLProfile as u32,
            NSOpenGLProfileVersion3_2Core as u32,
            0,
        ];

        println!("attributes: {:?}", attributes);

        let pixelformat = NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attributes);
        let cxt: id = NSOpenGLContext::alloc(nil).initWithFormat_shareContext_(pixelformat, nil);
        msg_send![cxt, setView:glview];
        let value = 1;
        cxt.setValues_forParameter_(&value, NSOpenGLContextParameter::NSOpenGLCPSwapInterval);
        CGLEnable(cxt.CGLContextObj() as *mut _, kCGLCECrashOnRemovedFunctions);
        cxt
    };

    unsafe {
        msg_send![cxt, update];
        msg_send![cxt, makeCurrentContext];
    };


    gleam::gl::load_with(|addr| {
        println!("get_proc_addr");
        let symbol_name: CFString = FromStr::from_str(addr).unwrap();
        let framework_name: CFString = FromStr::from_str("com.apple.opengl").unwrap();
        let framework = unsafe { CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef()) };
        let symbol = unsafe { CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef()) };
        symbol as *const c_void
    });

    // FIXME: release cxt
    let cxt_ptr = Box::into_raw(Box::new(cxt));
    // necessary?
    gleam::gl::clear_color(1.0, 0.0, 0.0, 1.0);
    gleam::gl::clear(gleam::gl::COLOR_BUFFER_BIT);
    // gleam::gl::finish();


    // let url = args().nth(1).unwrap_or("http://servo.org".to_owned());
    let url = "http://servo.org".to_owned();
    let servo = Servo::new(DrawableGeometry { inner_size: (200, 200), position: (0, 0), hidpi_factor: 1.0, },
                           EventLoopRiser {},
                           &url,
                           FollowLinkPolicy::FollowOriginalDomain);

    let servo_ptr = Box::into_raw(Box::new(servo));

    unsafe {
        let delegate = app_delegate::new_app_delegate();
        (*delegate).set_ivar("context", cxt_ptr as *mut c_void);
        (*delegate).set_ivar("servo", servo_ptr as *mut c_void);
        msg_send![app, setDelegate:delegate];
    }

    unsafe {
        app.run();
    }
}
