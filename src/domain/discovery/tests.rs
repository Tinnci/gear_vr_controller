use super::*;
fn packet(address: u64, name: &str, rssi: i16, matches_service: bool) -> Advertisement {
    Advertisement {
        address,
        name: name.into(),
        rssi,
        matches_service,
        address_kind: BluetoothAddressKind::Public,
    }
}
#[test]
fn joins_name_and_service_fragments_without_erasing_names() {
    let now = Instant::now();
    let mut catalog = DiscoveryCatalog::default();
    catalog.begin(vec![], None, false, now);
    catalog.observe(packet(1, "Gear VR Controller", -60, false), now);
    assert!(catalog.snapshot(now).is_empty());
    catalog.observe(packet(1, "", -64, true), now);
    catalog.observe(packet(1, "Gear", -60, false), now);
    let devices = catalog.snapshot(now);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name, "Gear VR Controller");
    assert!(devices[0].matches_service);
    // A scan response without UUIDs still supplies the controller's name.
    catalog.observe(packet(2, "", -75, true), now);
    catalog.observe(packet(2, "Second controller", -72, false), now);
    assert_eq!(catalog.snapshot(now)[1].name, "Second controller");
}
#[test]
fn rssi_updates_preserve_rows_until_one_final_ranking() {
    let now = Instant::now();
    let mut catalog = DiscoveryCatalog::default();
    catalog.begin(vec![3], Some(3), false, now);
    catalog.observe(packet(1, "Z", -70, true), now);
    catalog.observe(packet(2, "A", -40, true), now);
    catalog.observe(packet(3, "", -90, false), now);
    catalog.observe(packet(1, "Z full", -50, false), now);
    let devices = catalog.snapshot(now);
    assert_eq!(
        devices.iter().map(|d| d.address).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(devices[0].signal_strength, -65);
    catalog.finish(now);
    assert_eq!(
        catalog
            .snapshot(now)
            .iter()
            .map(|d| d.address)
            .collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
}
#[test]
fn silence_and_out_of_range_mark_unavailable_without_row_removal() {
    let now = Instant::now();
    let mut catalog = DiscoveryCatalog::default();
    catalog.begin(vec![], None, false, now);
    catalog.observe(packet(1, "", -50, true), now);
    catalog.observe(packet(1, "", -127, true), now);
    let devices = catalog.snapshot(now);
    assert_eq!(devices[0].signal_strength, -50);
    assert!(!devices[0].available);
    catalog.observe(packet(1, "", -55, true), now);
    assert!(!catalog.snapshot(now + Duration::from_secs(9))[0].available);
    assert_eq!(catalog.snapshot(now + Duration::from_secs(9)).len(), 1);
}
#[test]
fn new_scan_keeps_identity_but_expires_cache_and_resets_signal() {
    let now = Instant::now();
    let mut catalog = DiscoveryCatalog::default();
    catalog.begin(vec![], None, false, now);
    catalog.observe(packet(1, "Controller", -30, true), now);
    let next = now + Duration::from_secs(20);
    catalog.begin(vec![], None, false, next);
    assert!(!catalog.snapshot(next)[0].available);
    catalog.observe(packet(1, "", -80, false), next);
    let device = &catalog.snapshot(next)[0];
    assert_eq!(device.name, "Controller");
    assert_eq!(device.signal_strength, -80);
    assert!(device.available);
    catalog.begin(vec![], None, false, next + Duration::from_secs(61));
    assert!(catalog.snapshot(next + Duration::from_secs(61)).is_empty());
}
#[test]
fn random_addresses_are_distinct_and_noise_cannot_crowd_out_controller() {
    let now = Instant::now();
    let mut catalog = DiscoveryCatalog::default();
    catalog.begin(vec![], None, true, now);
    catalog.observe(packet(1, "Same name", -50, true), now);
    let mut random = packet(1, "Same name", -50, true);
    random.address_kind = BluetoothAddressKind::Random;
    catalog.observe(random, now);
    for address in 2..200 {
        catalog.observe(packet(address, "Noise", -50, false), now);
    }
    catalog.observe(packet(300, "", -80, true), now);
    let devices = catalog.snapshot(now);
    assert_eq!(devices.len(), CAPACITY);
    assert_eq!(
        devices.iter().filter(|device| device.address == 1).count(),
        2
    );
    assert!(devices.iter().any(|device| device.address == 300));
}
