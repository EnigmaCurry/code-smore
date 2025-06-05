use crate::debug;
use anyhow::Context;
use serialport::SerialPort;
use std::time::Duration;

/// RAII guard that asserts RTS on construction and de-asserts on drop.
pub struct RtsGuard {
    port: Box<dyn SerialPort>,
}

impl RtsGuard {
    pub fn new(port_name: &str) -> anyhow::Result<Self> {
        let mut port = serialport::new(port_name, 9_600)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("opening serial port `{}`", port_name))?;
        port.write_request_to_send(true).context("asserting RTS")?;
        debug!("RTS ON");
        Ok(RtsGuard { port })
    }
}

pub struct RtsCleanupGuard {
    port: Box<dyn SerialPort>,
}

impl RtsCleanupGuard {
    pub fn new_deassert_only(port_name: &str) -> anyhow::Result<Self> {
        let port = serialport::new(port_name, 9_600)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("opening serial port `{}`", port_name))?;
        Ok(Self { port })
    }
}

impl Drop for RtsCleanupGuard {
    fn drop(&mut self) {
        let _ = self.port.write_request_to_send(false); // Always clean up
        debug!("RTS OFF (deassert cleanup)");
    }
}

#[cfg(feature = "audio")]
impl Drop for RtsGuard {
    fn drop(&mut self) {
        // best-effort deassert
        let _ = self.port.write_request_to_send(false);
        debug!("RTS OFF");
    }
}

pub fn ensure_rts_deasserted(port_name: &str) -> anyhow::Result<()> {
    let mut port = serialport::new(port_name, 9600)
        .timeout(std::time::Duration::from_millis(100))
        .open()
        .with_context(|| format!("opening RTS port `{}`", port_name))?;

    port.write_request_to_send(false)?; // Deassert RTS immediately
    Ok(())
}
