pub fn get_battery() -> String {
    let manager = battery::Manager::new().unwrap();
    let battery = manager.batteries().unwrap().next().unwrap().unwrap();
    let percentage = battery.state_of_charge().value * 100.0;
    format!("{}%", percentage)
}
