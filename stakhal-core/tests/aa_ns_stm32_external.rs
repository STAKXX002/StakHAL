use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_aa_ns_stm32_external() {
    let test_dir = match std::env::var("STAKHAL_TEST_AA_NS_STM32") {
        Ok(val) => {
            let p = PathBuf::from(val);
            if !p.is_dir() {
                eprintln!(
                    "skipping test_aa_ns_stm32_external: STAKHAL_TEST_AA_NS_STM32 not set or path missing"
                );
                return;
            }
            p
        }
        Err(_) => {
            eprintln!(
                "skipping test_aa_ns_stm32_external: STAKHAL_TEST_AA_NS_STM32 not set or path missing"
            );
            return;
        }
    };

    let ioc_path = test_dir.join("AA_STM.ioc");
    let main_c_path = test_dir.join("Core/Src/main.c");

    let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
        .expect("Failed to load AA_NS_STM32 project");

    // 1. Assert 33 pins
    assert_eq!(project.pins.len(), 33, "expected 33 pins");

    // 2. Assert peripherals contain IWDG, TIM3 and USART2 and no GPIO
    let periph_names: Vec<String> = project.peripherals.iter().map(|p| p.name.clone()).collect();
    assert!(
        periph_names.contains(&"IWDG".to_string()),
        "expected IWDG peripheral in {:?}",
        periph_names
    );
    assert!(
        periph_names.contains(&"TIM3".to_string()),
        "expected TIM3 peripheral in {:?}",
        periph_names
    );
    assert!(
        periph_names.contains(&"USART2".to_string()),
        "expected USART2 peripheral in {:?}",
        periph_names
    );
    assert!(
        !periph_names.contains(&"GPIO".to_string()),
        "expected no GPIO peripheral in {:?}",
        periph_names
    );

    // 3. Pin to modules mapping table
    let mut expected_map: HashMap<&str, Vec<&str>> = HashMap::new();

    // axis pins
    for pin in &[
        "PA0", "PA1", "PA4", "PB0", "PC1", "PA9", "PC7", "PB6", "PA7", "PA6", "PA10", "PB5", "PB4",
        "PB10", "PA8", "PC6", "PC5", "PC8", "PC9", "PB8",
    ] {
        expected_map.insert(pin, vec!["axis"]);
    }

    // relay, system
    expected_map.insert("PB7", vec!["relay", "system"]);

    // hatch, system
    expected_map.insert("PB12", vec!["hatch", "system"]);
    expected_map.insert("PB13", vec!["hatch", "system"]);
    expected_map.insert("PB14", vec!["hatch", "system"]);
    expected_map.insert("PB15", vec!["hatch", "system"]);

    // commands
    expected_map.insert("PA2", vec!["commands"]);
    expected_map.insert("PA3", vec!["commands"]);

    // no modules
    for pin in &["PA5", "PA13", "PA14", "PC13", "PH0", "PH1"] {
        expected_map.insert(pin, vec![]);
    }

    assert_eq!(
        expected_map.len(),
        33,
        "expected map must cover all 33 pins"
    );

    let actual_map: HashMap<String, Vec<String>> = project
        .pins
        .iter()
        .map(|p| (p.pin.clone(), p.modules.clone()))
        .collect();

    for (pin, expected_mods) in expected_map {
        let actual_mods = actual_map.get(pin).unwrap_or_else(|| {
            panic!("pin {} missing from project pins", pin);
        });
        assert_eq!(
            actual_mods.as_slice(),
            expected_mods.as_slice(),
            "mismatch for pin {}",
            pin
        );
    }
}
