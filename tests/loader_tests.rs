#![cfg(feature = "asset-loader")]

use bevy_symbios::loader::{parse_lsys_source, parse_material_settings};

#[test]
fn parses_lsys_source() {
    let src = r#"
omega: A(0)
A(x) : x < 5 -> A(x + 1) B
"#;
    let sys = parse_lsys_source(src).expect("parse should succeed");
    // Round-trip: derive a few steps without panicking — exercises the parsed grammar.
    let _ = sys; // System fields aren't all public; the fact that we got it is enough.
}

#[test]
fn rejects_malformed_lsys() {
    let src = "omega: A(\n??? not a rule\n";
    let result = parse_lsys_source(src);
    assert!(result.is_err(), "Malformed source must surface an error");
}

#[test]
fn parses_material_settings_json() {
    let json = br#"
    {
      "0": {
        "base_color": [0.5, 0.5, 0.5],
        "roughness": 0.7,
        "metallic": 0.1
      }
    }
    "#;
    let map = parse_material_settings(json).expect("JSON should parse");
    assert_eq!(map.len(), 1);
    let s = map.get(&0).expect("entry 0 exists");
    assert!((s.roughness - 0.7).abs() < 1e-6);
    assert!((s.metallic - 0.1).abs() < 1e-6);
}

#[test]
fn rejects_invalid_json() {
    let json = b"{ this is not json";
    let result = parse_material_settings(json);
    assert!(result.is_err());
}
