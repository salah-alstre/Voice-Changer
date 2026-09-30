#![allow(non_upper_case_globals)]
//! Windows Core Audio layer.

pub mod devices;
pub mod engine;
pub mod icons;
pub mod policy;
pub mod recorder;
pub mod sessions;
pub mod stream;

#[cfg(test)]
mod hardware_tests {
    //! Real-hardware smoke tests. Run with `cargo test --lib -- --ignored --nocapture`.
    use super::*;

    #[test]
    #[ignore = "needs real audio endpoints"]
    fn lists_real_devices_and_sessions() {
        for flow in [devices::Flow::Render, devices::Flow::Capture] {
            let list = devices::list_devices(flow).expect("enumerate");
            println!("--- {flow:?}: {} device(s)", list.len());
            for d in &list {
                println!(
                    "{} | {} | def={} | {}Hz {}ch {}bit | vol={:.2} mute={} virt={} | {}",
                    d.name,
                    d.state,
                    d.is_default,
                    d.sample_rate,
                    d.channels,
                    d.bits,
                    d.volume,
                    d.muted,
                    d.is_virtual,
                    d.form_factor
                );
            }
        }
        let mut p = sessions::SessionPoller::new(None).expect("session poller");
        let s = p.poll().expect("poll");
        println!("--- {} session(s)", s.len());
        for x in s {
            println!(
                "{} pid={} {} vol={:.2} peak={:.3}",
                x.name, x.pid, x.state, x.volume, x.peak
            );
        }
    }
}
