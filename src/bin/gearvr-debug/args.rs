use anyhow::{bail, ensure, Result};
use windows::Devices::Bluetooth::BluetoothAddressType;

pub const USAGE: &str = "gearvr-debug <status|scan|pair|connect> [--address AABBCCDDEEFF] [--address-type public|random|auto] [--seconds 15] [--timeout 45] [--all]\nstatus: adapter and optional device state; scan: final stable device list; pair: Windows pairing; connect: protocol initialization and bounded packet capture.\nPair and connect require an explicit address. Close the GUI before probing. No desktop input is sent. Stdout contains JSON lines with local device identifiers.";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Status,
    Scan,
    Pair,
    Connect,
}
impl Command {
    pub fn name(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Scan => "scan",
            Self::Pair => "pair",
            Self::Connect => "connect",
        }
    }
}
pub struct Args {
    pub command: Command,
    pub address: Option<u64>,
    pub address_type: Option<BluetoothAddressType>,
    pub seconds: u64,
    pub timeout: u64,
    pub all: bool,
}
impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>> {
        let mut items = args.into_iter();
        let Some(command) = items.next() else {
            return Ok(None);
        };
        if command == "--help" || command == "-h" {
            return Ok(None);
        }
        let command = match command.as_str() {
            "status" => Command::Status,
            "scan" => Command::Scan,
            "pair" => Command::Pair,
            "connect" => Command::Connect,
            _ => bail!("Unknown command. {}", USAGE),
        };
        let mut result = Self {
            command,
            address: None,
            address_type: None,
            seconds: 15,
            timeout: 45,
            all: false,
        };
        let mut seen = std::collections::HashSet::new();
        while let Some(option) = items.next() {
            ensure!(seen.insert(option.clone()), "Duplicate option: {option}");
            if option == "--all" {
                result.all = true;
                continue;
            }
            let value = items
                .next()
                .ok_or_else(|| anyhow::anyhow!("Missing value for {option}"))?;
            match option.as_str() {
                "--address" => result.address = Some(parse_address(&value)?),
                "--address-type" => {
                    result.address_type = match value.as_str() {
                        "public" => Some(BluetoothAddressType::Public),
                        "random" => Some(BluetoothAddressType::Random),
                        "auto" => None,
                        _ => bail!("Address type must be public, random or auto"),
                    }
                }
                "--seconds" => result.seconds = value.parse()?,
                "--timeout" => result.timeout = value.parse()?,
                _ => bail!("Unknown option: {option}"),
            }
        }
        ensure!(
            (1..=120).contains(&result.seconds),
            "Seconds must be between 1 and 120"
        );
        ensure!(
            (3..=180).contains(&result.timeout),
            "Timeout must be between 3 and 180 seconds"
        );
        ensure!(
            !matches!(command, Command::Pair | Command::Connect) || result.address.is_some(),
            "Pair and connect require --address"
        );
        ensure!(
            !result.all || command == Command::Scan,
            "--all is only valid for scan"
        );
        ensure!(
            command != Command::Scan
                || (result.address.is_none() && !seen.contains("--address-type")),
            "Scan does not accept an address"
        );
        ensure!(
            result.address.is_some() || !seen.contains("--address-type"),
            "Address type requires --address"
        );
        ensure!(
            !seen.contains("--seconds") || matches!(command, Command::Scan | Command::Connect),
            "--seconds is only valid for scan and connect"
        );
        ensure!(
            !matches!(command, Command::Scan | Command::Connect) || result.seconds < result.timeout,
            "Timeout must exceed capture duration"
        );
        Ok(Some(result))
    }
}

fn parse_address(value: &str) -> Result<u64> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    let compact = value.replace([':', '-'], "");
    ensure!(
        compact.len() == 12 && compact.bytes().all(|b| b.is_ascii_hexdigit()),
        "Address must contain 12 hexadecimal digits"
    );
    let address = u64::from_str_radix(&compact, 16)?;
    ensure!(address > 0, "Zero is not a device address");
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(value: &str) -> Result<Option<Args>> {
        Args::parse(value.split_whitespace().map(String::from))
    }
    #[test]
    fn mutations_require_explicit_valid_target() {
        for value in [
            "pair",
            "connect",
            "pair --address 0",
            "pair --address 000000000000",
            "connect --address FFFFFFFFFFFFF",
            "pair --address xyz",
        ] {
            assert!(parse(value).is_err());
        }
        assert_eq!(
            parse_address("2C:BA:BA:2F:51:B0").ok(),
            Some(0x2CBABA2F51B0)
        );
    }
    #[test]
    fn reject_ambiguous_or_unbounded_options() {
        for value in [
            "scan --seconds 0",
            "scan --timeout 1000",
            "scan --seconds 45 --timeout 45",
            "pair --all",
            "status --address-type random",
            "status --seconds 5",
            "scan --address 123456789ABC",
            "scan --seconds 5 --seconds 6",
            "scan --wat 1",
        ] {
            assert!(parse(value).is_err(), "{value}");
        }
    }
    #[test]
    fn public_and_random_addresses_are_preserved() -> Result<()> {
        for (name, kind) in [
            ("public", BluetoothAddressType::Public),
            ("random", BluetoothAddressType::Random),
        ] {
            let args = parse(&format!(
                "connect --address 123456789ABC --address-type {name}"
            ))?
            .ok_or_else(|| anyhow::anyhow!("Missing args"))?;
            assert_eq!(args.address_type, Some(kind));
        }
        Ok(())
    }
}
