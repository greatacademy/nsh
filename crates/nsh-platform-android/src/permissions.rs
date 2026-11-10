//! Java for an app's own Rust crate: the VM, the application's context and
//! runtime permissions (`Permissions.java`), as the microphone for audio the
//! crate captures itself with `AudioRecord`. The crate declares the
//! permission in the app's manifest (`nsh package` writes `RECORD_AUDIO`
//! for apps granted `speech`).
//!
//! The crate uses the [`jni`] this crate re-exports, so that the two agree
//! on its types.

use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicI64, Ordering},
    },
};

use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JClass, JValue},
    sys::{jboolean, jlong},
};

use crate::java::{java, with_env};

type Answer = Box<dyn FnOnce(bool) + Send>;

static NEXT: AtomicI64 = AtomicI64::new(1);
static WAITING: Mutex<Option<HashMap<i64, Answer>>> = Mutex::new(None);

/// The Java VM the app runs in, to attach threads of the crate's own.
pub fn java_vm() -> &'static JavaVM {
    &java().vm
}

/// The application's context, once the app's view was made.
pub fn context() -> Option<GlobalRef> {
    with_env(|env| {
        let class: &JClass<'_> = java().permissions.as_obj().into();
        let context = env.call_static_method(class, "context", "()Landroid/content/Context;", &[])?.l()?;
        if context.is_null() {
            return Ok(None);
        }
        Ok(Some(env.new_global_ref(context)?))
    })
}

/// Whether the app has `permission` (`android.permission.RECORD_AUDIO`).
pub fn granted(permission: &str) -> bool {
    with_env(|env| {
        let class: &JClass<'_> = java().permissions.as_obj().into();
        let name = env.new_string(permission)?;
        env.call_static_method(class, "granted", "(Ljava/lang/String;)Z", &[JValue::Object(&name)])?.z()
    })
}

/// Asks for `permission` with the system's prompt when the app does not
/// have it; `answer` gets whether it does now, on the UI thread.
pub fn request_permission(permission: &str, answer: impl FnOnce(bool) + Send + 'static) {
    let token = NEXT.fetch_add(1, Ordering::Relaxed);
    WAITING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(token, Box::new(answer));
    let sent = with_env(|env| {
        let class: &JClass<'_> = java().permissions.as_obj().into();
        let name = env.new_string(permission)?;
        env.call_static_method(class, "request", "(Ljava/lang/String;J)V", &[JValue::Object(&name), JValue::Long(token)])?;
        Ok(true)
    });
    if !sent {
        answered(token, false);
    }
}

fn answered(token: i64, granted: bool) {
    let answer = WAITING.lock().unwrap_or_else(|e| e.into_inner()).as_mut().and_then(|w| w.remove(&token));
    if let Some(answer) = answer {
        answer(granted);
    }
}

/// `Permissions.answer`: the system's answer to a request.
#[unsafe(no_mangle)]
extern "system" fn Java_com_longbridge_nsh_Permissions_answer(_: JNIEnv<'_>, _: JClass<'_>, token: jlong, granted: jboolean) {
    answered(token, granted != 0);
}
