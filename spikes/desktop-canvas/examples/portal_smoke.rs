//! Exercises the real D-Bus request/signal contract against a private fake portal.
//! Run with `dbus-run-session -- cargo run -p hoshi-canvas-spike --example portal_smoke`.
#[cfg(target_os = "linux")]
#[path = "../src/screen_pick.rs"]
mod screen_pick;

#[cfg(target_os = "linux")]
fn main() {
    use hoshi_controller::Color;
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU32, Ordering},
        },
        time::Duration,
    };
    use zbus::{
        Connection,
        message::Header,
        zvariant::{OwnedObjectPath, Value},
    };

    struct FakePortal(Arc<AtomicU32>, Arc<AtomicBool>);
    struct FakeRequest(Arc<AtomicBool>);
    #[zbus::interface(name = "org.freedesktop.portal.Request")]
    impl FakeRequest {
        fn close(&self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    #[zbus::interface(name = "org.freedesktop.portal.Screenshot")]
    impl FakePortal {
        async fn pick_color(
            &self,
            _parent: &str,
            options: HashMap<&str, Value<'_>>,
            #[zbus(header)] header: Header<'_>,
            #[zbus(connection)] connection: &Connection,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            let sender = header.sender().expect("sender").as_str();
            let token = <&str>::try_from(options.get("handle_token").expect("token"))
                .expect("string token");
            let path = OwnedObjectPath::try_from(format!(
                "/org/freedesktop/portal/desktop/request/{}/{}",
                sender.trim_start_matches(':').replace('.', "_"),
                token
            ))
            .unwrap();
            let mode = self.0.load(Ordering::Relaxed);
            if mode == 5 {
                connection
                    .object_server()
                    .at(path.as_str(), FakeRequest(self.1.clone()))
                    .await
                    .unwrap();
                return Ok(path);
            }
            let mut values = HashMap::new();
            if mode == 0 {
                values.insert("color", Value::from((0.0f64, 0.5f64, 1.0f64)));
            }
            if mode == 4 {
                values.insert("color", Value::from((f64::NAN, 0.0f64, 0.0f64)));
            }
            // Respond BEFORE the method reply to verify race-free subscription.
            connection
                .emit_signal(
                    Some(sender),
                    path.as_str(),
                    "org.freedesktop.portal.Request",
                    "Response",
                    &(if mode >= 3 { 0 } else { mode }, values),
                )
                .await
                .unwrap();
            Ok(path)
        }
    }
    let mode = Arc::new(AtomicU32::new(0));
    let closed = Arc::new(AtomicBool::new(false));
    let _server = futures_lite::future::block_on(async {
        zbus::connection::Builder::session()
            .unwrap()
            .name("org.freedesktop.portal.Desktop")
            .unwrap()
            .serve_at(
                "/org/freedesktop/portal/desktop",
                FakePortal(mode.clone(), closed.clone()),
            )
            .unwrap()
            .build()
            .await
            .unwrap()
    });
    let result = |mode_value| {
        mode.store(mode_value, Ordering::Relaxed);
        screen_pick::start()
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("portal response")
    };
    assert_eq!(result(0).unwrap(), Some(Color::rgba(0, 128, 255, 255)));
    assert_eq!(result(1).unwrap(), None);
    assert!(result(2).is_err());
    assert!(result(3).is_err(), "missing color must fail");
    assert!(result(4).is_err(), "invalid color must fail");
    mode.store(5, Ordering::Relaxed);
    let operation = screen_pick::start();
    std::thread::sleep(Duration::from_millis(100));
    drop(operation);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !closed.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        closed.load(Ordering::Relaxed),
        "drop must close portal request"
    );
    println!(
        "PASS: real portal request/signal contract, immediate color, cancellation, denial, malformed result, request cleanup"
    );
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("Linux-only desktop portal contract check");
}
