use async_trait::async_trait;
use bytes::Bytes;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::domain::{BootDev, IpmiCreds, PowerStatus, Sensors};
use crate::error::{AppError, Result};
use crate::ports::ipmi::{IpmiController, SolSession};

pub struct IpmitoolController {
    binary: String,
    interface: String,
}

impl IpmitoolController {
    pub fn new() -> Self {
        IpmitoolController {
            binary: "ipmitool".to_string(),
            interface: "lanplus".to_string(),
        }
    }

    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = interface.into();
        self
    }

    fn base_command(&self, creds: &IpmiCreds) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("-I")
            .arg(&self.interface)
            .arg("-H")
            .arg(&creds.host)
            .arg("-U")
            .arg(&creds.username)
            .arg("-P")
            .arg(&creds.password)
            .arg("-C")
            .arg(creds.cipher.to_string());
        cmd
    }

    async fn run(&self, creds: &IpmiCreds, args: &[&str]) -> Result<String> {
        let output = self
            .base_command(creds)
            .args(args)
            .output()
            .await
            .map_err(|e| AppError::Ipmi(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.trim();
            return Err(AppError::Ipmi(if msg.is_empty() {
                format!("ipmitool exited with {}", output.status)
            } else {
                msg.to_string()
            }));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Default for IpmitoolController {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IpmiController for IpmitoolController {
    async fn power_status(&self, creds: &IpmiCreds) -> Result<PowerStatus> {
        let out = self.run(creds, &["chassis", "power", "status"]).await?;
        Ok(parse_power_status(&out))
    }

    async fn power_on(&self, creds: &IpmiCreds) -> Result<()> {
        self.run(creds, &["chassis", "power", "on"]).await?;
        Ok(())
    }

    async fn power_off(&self, creds: &IpmiCreds) -> Result<()> {
        self.run(creds, &["chassis", "power", "off"]).await?;
        Ok(())
    }

    async fn power_cycle(&self, creds: &IpmiCreds) -> Result<()> {
        self.run(creds, &["chassis", "power", "cycle"]).await?;
        Ok(())
    }

    async fn set_bootdev(&self, creds: &IpmiCreds, dev: BootDev) -> Result<()> {
        self.run(creds, &["chassis", "bootdev", dev.as_ipmitool()])
            .await?;
        Ok(())
    }

    async fn sensors(&self, creds: &IpmiCreds) -> Result<Sensors> {
        let power_status = self.power_status(creds).await?;
        let (power_w, cpu_temp_c) = match self.run(creds, &["sdr"]).await {
            Ok(out) => parse_sdr(&out),
            Err(_) => (None, None),
        };
        Ok(Sensors {
            power_status,
            power_w,
            cpu_temp_c,
        })
    }

    async fn sol_console(&self, creds: &IpmiCreds) -> Result<Box<dyn SolSession>> {
        let mut child = self
            .base_command(creds)
            .arg("sol")
            .arg("activate")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::Ipmi(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Ipmi("sol: no stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Ipmi("sol: no stdout".to_string()))?;
        Ok(Box::new(IpmitoolSol {
            child,
            stdin,
            stdout,
            binary: self.binary.clone(),
            interface: self.interface.clone(),
            creds: creds.clone(),
        }))
    }
}

struct IpmitoolSol {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    binary: String,
    interface: String,
    creds: IpmiCreds,
}

#[async_trait]
impl SolSession for IpmitoolSol {
    async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.stdin
            .write_all(data)
            .await
            .map_err(|e| AppError::Ipmi(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| AppError::Ipmi(e.to_string()))?;
        Ok(())
    }

    async fn read(&mut self) -> Result<Option<Bytes>> {
        let mut buf = vec![0u8; 4096];
        let n = self
            .stdout
            .read(&mut buf)
            .await
            .map_err(|e| AppError::Ipmi(e.to_string()))?;
        if n == 0 {
            Ok(None)
        } else {
            buf.truncate(n);
            Ok(Some(Bytes::from(buf)))
        }
    }

    async fn close(mut self: Box<Self>) -> Result<()> {
        let _ = self.child.kill().await;
        let _ = Command::new(&self.binary)
            .arg("-I")
            .arg(&self.interface)
            .arg("-H")
            .arg(&self.creds.host)
            .arg("-U")
            .arg(&self.creds.username)
            .arg("-P")
            .arg(&self.creds.password)
            .arg("-C")
            .arg(self.creds.cipher.to_string())
            .arg("sol")
            .arg("deactivate")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await;
        Ok(())
    }
}

fn parse_power_status(out: &str) -> PowerStatus {
    let lower = out.to_ascii_lowercase();
    if lower.contains("is on") {
        PowerStatus::On
    } else if lower.contains("is off") {
        PowerStatus::Off
    } else {
        PowerStatus::Unknown
    }
}

fn extract_number(s: &str) -> Option<f64> {
    let mut buf = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' || (c == '-' && buf.is_empty()) {
            buf.push(c);
        } else if !buf.is_empty() {
            break;
        }
    }
    buf.parse().ok()
}

fn parse_sdr(out: &str) -> (Option<f64>, Option<f64>) {
    let mut power_w: Option<f64> = None;
    let mut power_priority = false;
    let mut cpu_temp_c: Option<f64> = None;
    let mut temp_priority = false;

    for line in out.lines() {
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() < 2 {
            continue;
        }
        let name = cols[0].trim().to_ascii_lowercase();
        let value = cols[1].trim();
        let value_lower = value.to_ascii_lowercase();

        let is_consumption = name.contains("pwr consumption") || name.contains("power");
        if value_lower.contains("watt")
            && is_consumption
            && (power_w.is_none() || (!power_priority && name.contains("pwr consumption")))
            && let Some(w) = extract_number(value)
        {
            power_w = Some(w);
            power_priority = name.contains("pwr consumption");
        }

        let is_cpu = name.contains("cpu");
        if name.contains("temp")
            && (value_lower.contains("degree") || value_lower.contains('c'))
            && (cpu_temp_c.is_none() || (!temp_priority && is_cpu))
            && let Some(t) = extract_number(value)
        {
            cpu_temp_c = Some(t);
            temp_priority = is_cpu;
        }
    }

    (power_w, cpu_temp_c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs() {
        let c = IpmitoolController::new()
            .with_binary("ipmitool")
            .with_interface("lanplus");
        assert_eq!(c.binary, "ipmitool");
        assert_eq!(c.interface, "lanplus");
    }

    #[test]
    fn power_status_on_off_unknown() {
        assert_eq!(
            parse_power_status("Chassis Power is on"),
            PowerStatus::On
        );
        assert_eq!(
            parse_power_status("Chassis Power is off\n"),
            PowerStatus::Off
        );
        assert_eq!(
            parse_power_status("Error: Unable to establish session"),
            PowerStatus::Unknown
        );
    }

    #[test]
    fn sdr_parses_power_and_temp() {
        let sample = "\
Pwr Consumption  | 140 Watts         | ok
CPU Temp         | 45 degrees C      | ok
Inlet Temp       | 22 degrees C      | ok
Fan1             | 4800 RPM          | ok
";
        let (w, t) = parse_sdr(sample);
        assert_eq!(w, Some(140.0));
        assert_eq!(t, Some(45.0));
    }

    #[test]
    fn sdr_prefers_cpu_temp_over_generic() {
        let sample = "\
Inlet Temp       | 22 degrees C      | ok
CPU Temp         | 50 degrees C      | ok
";
        let (_w, t) = parse_sdr(sample);
        assert_eq!(t, Some(50.0));
    }

    #[test]
    fn sdr_absent_sensors_are_none() {
        let sample = "Fan1 | 4800 RPM | ok\nVoltage | 12 V | ok\n";
        let (w, t) = parse_sdr(sample);
        assert_eq!(w, None);
        assert_eq!(t, None);
    }

    #[test]
    fn sdr_falls_back_to_power_label() {
        let sample = "System Power | 95 Watts | ok\n";
        let (w, _t) = parse_sdr(sample);
        assert_eq!(w, Some(95.0));
    }
}
